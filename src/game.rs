use async_std::io;
use async_std::sync::{Arc, Mutex};

use futures::prelude::*;

use async_tungstenite::tungstenite::protocol::Message;
use tungstenite::error::Error as WsError;

use std::io::{Error as IoError, ErrorKind};

use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::json;

enum SC3Op {
    Linebreak(usize),
    RubyBase(usize),
    RubyEnd(usize),
}

impl Serialize for SC3Op {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        let (op, offset) = match self {
            SC3Op::Linebreak(o) => ("linebreak", o),
            SC3Op::RubyBase(o) => ("ruby-base", o),
            SC3Op::RubyEnd(o) => ("ruby-end", o),
        };
        map.serialize_entry("op", op)?;
        map.serialize_entry("offset", offset)?;
        map.end()
    }
}

#[derive(Serialize)]
pub struct SC3String {
    content: String,
    markers: Vec<SC3Op>,
}

impl SC3String {
    const CHARSET: &'static str = include_str!("../res/charset.utf8");

    async fn read_from_stdin(stdin: &mut io::Stdin) -> Result<SC3String, IoError> {
        let mut reached_expression_end = false;
        let mut content: String = String::new();
        let mut markers: Vec<SC3Op> = Vec::new();

        while !reached_expression_end {
            let mut token = [0u8; 1];
            stdin.read_exact(&mut token).await?;
            match token[0] {
                0x00 => markers.push(SC3Op::Linebreak(content.len())),
                0x09 => markers.push(SC3Op::RubyBase(content.len())),
                0x0B => markers.push(SC3Op::RubyEnd(content.len())),
                0x80..=0xFE => {
                    let mut char_lower_half = [0u8; 1];
                    stdin.read_exact(&mut char_lower_half).await?;
                    let codepoint: usize =
                        (((token[0] as usize) << 8) | (char_lower_half[0] as usize)) - 0x8000;
                    if let Some(character) = SC3String::CHARSET.chars().nth(codepoint) {
                        if character == '\u{3000}' {
                            content.push(' ');
                        } else {
                            content.push(character);
                        }
                    } else {
                        return Err(IoError::new(ErrorKind::InvalidData, "Unknown codepoint"));
                    }
                }
                0xFF => reached_expression_end = true,
                _ => return Err(IoError::new(ErrorKind::InvalidData, "Unknown string token")),
            }
        }

        Ok(SC3String { content, markers })
    }
}

#[derive(Serialize)]
pub struct Tweep {
    pub id: u32,
    pub tab: u16,
    pub pfp_id: u16,
    pub different_day: bool,
    pub author_username: SC3String,
    pub author_realname: SC3String,
    pub content: SC3String,
    pub replies: Vec<SC3String>,
    pub reply_possible: bool,
}

impl Tweep {
    async fn read_from_stdin(stdin: &mut io::Stdin) -> Result<Tweep, IoError> {
        let mut id_buf = [0u8; 4];
        stdin.read_exact(&mut id_buf).await?;
        let mut tab_buf = [0u8; 2];
        stdin.read_exact(&mut tab_buf).await?;
        let mut pfp_id_buf = [0u8; 2];
        stdin.read_exact(&mut pfp_id_buf).await?;
        let mut different_day_buf = [0u8; 2];
        stdin.read_exact(&mut different_day_buf).await?;
        let mut replies_amount_buf = [0u8; 2];
        stdin.read_exact(&mut replies_amount_buf).await?;

        let author_username = SC3String::read_from_stdin(stdin).await?;
        let author_realname = SC3String::read_from_stdin(stdin).await?;
        let content = SC3String::read_from_stdin(stdin).await?;

        let replies_amount = u16::from_ne_bytes(replies_amount_buf);
        let mut replies = Vec::with_capacity(replies_amount as usize);
        for _ in 0..replies_amount {
            replies.push(SC3String::read_from_stdin(stdin).await?);
        }

        Ok(Tweep {
            id: u32::from_ne_bytes(id_buf),
            tab: u16::from_ne_bytes(tab_buf),
            pfp_id: u16::from_ne_bytes(pfp_id_buf),
            different_day: u16::from_ne_bytes(different_day_buf) != 0,
            author_username,
            author_realname,
            content,
            replies,
            reply_possible: false,
        })
    }
}

pub type Tweeps = Arc<Mutex<Vec<Tweep>>>;

pub async fn read_stdin(
    write_streams: super::http::WriteStreams,
    tweeps: Tweeps,
) -> Result<(), IoError> {
    let mut stdin = io::stdin();

    loop {
        let mut message_type_buf = [0u8; 4];
        stdin.read_exact(&mut message_type_buf).await?;
        let next_message = match u32::from_ne_bytes(message_type_buf) {
            0x434c4541 => {
                // "CLEA" : Clear
                tweeps.lock().await.clear();
                json!({"type": "clear"}).to_string()
            }
            0x54574550 => {
                // "TWEP" : Tweep
                let tweep = Tweep::read_from_stdin(&mut stdin).await?;
                let tweep_as_json = json!({"type": "tweep", "tweep": tweep}).to_string();
                tweeps.lock().await.push(tweep);
                tweep_as_json
            }
            0x53545250 => {
                // "STRP" : Set Reply Possible
                let mut id_buf = [0u8; 4];
                stdin.read_exact(&mut id_buf).await?;
                let id = u32::from_ne_bytes(id_buf);
                let mut possible_buf = [0u8; 2];
                stdin.read_exact(&mut possible_buf).await?;
                let possible = u16::from_ne_bytes(possible_buf) != 0;

                let mut locked_tweeps = tweeps.lock().await;
                let tweep = match locked_tweeps.iter_mut().find(|tweep| tweep.id == id) {
                    Some(t) => t,
                    // The game may send STRP messages for tweeps we don't know about (e.g. during
                    // game init where it will go through all tweeps), so we'll ignore them.
                    None => continue,
                };
                tweep.reply_possible = possible;

                json!({
                       "type": "set_reply_possible",
                       "tweep_id": id,
                       "possible": possible,
                })
                .to_string()
            }
            _ => {
                panic!("Unknown message type from game : possible desync !");
            }
        };

        let mut index_to_remove: Vec<usize> = Vec::new();
        let mut locked_write_streams = write_streams.lock().await;
        for (index, stream) in locked_write_streams.iter_mut().enumerate() {
            if stream.send(Message::text(&next_message)).await.is_err() {
                index_to_remove.push(index);
            }
        }
        for index in index_to_remove.iter().rev() {
            match locked_write_streams.remove(*index).close().await {
                Ok(_) | Err(WsError::ConnectionClosed) => (),
                Err(error) => eprintln!("Unable to close sink : {}", error),
            }
        }
    }
}

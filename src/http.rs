use async_std::io;
use async_std::net::{SocketAddr, TcpListener, TcpStream};
use async_std::sync::{Arc, Mutex};
use async_std::task;

use futures::prelude::*;

use std::error::Error;
use std::io::{Error as IoError, ErrorKind};

use sha1::{Digest, Sha1};

use async_tungstenite::tungstenite::protocol::Message;

use serde::Deserialize;
use serde_json::json;

use super::game::Tweeps;
use super::images::ImageList;

struct HttpError {
    code: u32,
    status: &'static str,
}

const HTTP_404: HttpError = HttpError {
    code: 404,
    status: "Not Found",
};
const HTTP_400: HttpError = HttpError {
    code: 400,
    status: "Bad Request",
};

pub type WriteStreams =
    Arc<Mutex<Vec<stream::SplitSink<async_tungstenite::WebSocketStream<TcpStream>, Message>>>>;

struct HttpConnection {
    stream: TcpStream,
    peer_addr: SocketAddr,
    write_streams: WriteStreams,
    tweeps: Tweeps,
    image_list: Arc<ImageList>,
}

impl HttpConnection {
    fn new(
        stream: TcpStream,
        peer_addr: SocketAddr,
        write_streams: WriteStreams,
        tweeps: Tweeps,
        image_list: Arc<ImageList>,
    ) -> HttpConnection {
        HttpConnection {
            stream,
            peer_addr,
            write_streams,
            tweeps,
            image_list,
        }
    }

    async fn handle_websocket(self) -> Result<(), Box<dyn Error>> {
        eprintln!("{} : WS Opened", self.peer_addr);
        let ws_stream = async_tungstenite::WebSocketStream::from_raw_socket(
            self.stream,
            tungstenite::protocol::Role::Server,
            None,
        )
        .await;
        let (mut write, mut read) = ws_stream.split();
        for tweep in self.tweeps.lock().await.iter() {
            let tweep_as_json = json!({"type": "tweep", "tweep": tweep}).to_string();
            write.send(Message::text(tweep_as_json)).await?;
        }
        self.write_streams.lock().await.push(write);

        while let Some(message) = read.next().await {
            let valid_message = message?;
            let message_str = valid_message.to_text()?;
            eprintln!("{} : {}", self.peer_addr, message_str.trim());

            #[derive(Deserialize)]
            struct TweepReply {
                r#type: String,
                tweep_id: u32,
                reply_id: u32,
            }
            let tweep_reply: TweepReply = serde_json::from_str(message_str)?;

            if tweep_reply.r#type != "reply" {
                return Err(Box::new(IoError::new(
                    ErrorKind::InvalidData,
                    "Invalid message type from websocket",
                )));
            }
            let locked_tweeps = self.tweeps.lock().await;
            let tweep = match locked_tweeps
                .iter()
                .find(|tweep| tweep.id == tweep_reply.tweep_id)
            {
                Some(t) => t,
                None => {
                    return Err(Box::new(IoError::new(
                        ErrorKind::InvalidData,
                        "Invalid tweep id from tweep reply via websocket",
                    )))
                }
            };
            if tweep_reply.reply_id as usize >= tweep.replies.len() {
                return Err(Box::new(IoError::new(
                    ErrorKind::InvalidData,
                    "Invalid reply id from tweep reply via websocket",
                )));
            }

            let message = [
                0x594c5052_u32.to_ne_bytes(),
                tweep_reply.tweep_id.to_ne_bytes(),
                tweep_reply.reply_id.to_ne_bytes(),
            ]
            .concat();
            io::stdout().write_all(&message).await?;
            io::stdout().flush().await?;
        }
        eprintln!("{} : WS Closed", self.peer_addr);
        Ok(())
    }

    async fn write_response(
        &mut self,
        code: u32,
        status: &str,
        content_type: &str,
        data: &[u8],
    ) -> Result<(), IoError> {
        let header = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
            code,
            status,
            content_type,
            data.len()
        );
        self.stream.write_all(header.as_bytes()).await?;
        self.stream.write_all(data).await?;
        Ok(())
    }

    async fn write_text_response(
        &mut self,
        code: u32,
        status: &str,
        content_type: &str,
        text: &str,
    ) -> Result<(), IoError> {
        self.write_response(
            code,
            status,
            &(content_type.to_owned() + "; charset=utf-8"),
            text.as_bytes(),
        )
        .await
    }

    async fn write_error(&mut self, error: &HttpError) -> Result<(), IoError> {
        self.write_text_response(
            error.code,
            error.status,
            "text/html",
            &format!("<h1>{} {}</h1>", error.code, error.status),
        )
        .await
    }

    async fn write_upgrade_response(&mut self, key: &[u8]) -> Result<(), IoError> {
        let to_hash = [key, b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"].concat();
        let mut hasher = Sha1::new();
        hasher.update(to_hash);
        let hash = hasher.finalize();
        let header = format!(
            "HTTP/1.1 101 Switching Protocols\r\nSec-WebSocket-Accept: {}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\r\n",
            base64::encode(hash)
        );
        self.stream.write_all(header.as_bytes()).await?;
        Ok(())
    }

    async fn handle_upgrade_request<'a>(
        &mut self,
        headers: &[httparse::Header<'a>],
    ) -> Result<(u32, bool), IoError> {
        let required_headers = [
            httparse::Header {
                name: "Upgrade",
                value: b"websocket",
            },
            httparse::Header {
                name: "Connection",
                value: b"Upgrade",
            },
            httparse::Header {
                name: "Sec-WebSocket-Version",
                value: b"13",
            },
        ];
        let mut valid = true;
        for required_header in required_headers.iter() {
            valid = match headers
                .iter()
                .find(|header| header.name.to_lowercase() == required_header.name.to_lowercase())
            {
                Some(h) => {
                    let expected = std::str::from_utf8(required_header.value)
                        .unwrap()
                        .to_lowercase();
                    match std::str::from_utf8(h.value) {
                        // We use `contains` because the values are not that strict
                        // e.g. `Connection: keep-alive, Upgrade` is possible
                        Ok(s) => s.to_lowercase().contains(&expected),
                        Err(_) => false,
                    }
                }
                None => false,
            };
            if !valid {
                break;
            }
        }

        if valid {
            valid = match headers
                .iter()
                .find(|header| header.name.to_lowercase() == "sec-websocket-key")
            {
                Some(h) => {
                    self.write_upgrade_response(h.value).await?;
                    true
                }
                None => false,
            };
        }

        if !valid {
            self.write_error(&HTTP_400).await?;
            Ok((400, valid))
        } else {
            Ok((101, valid))
        }
    }

    async fn handle_connection(mut self) -> Result<(), Box<dyn Error>> {
        let mut request_buffer: Vec<u8> = Vec::new();
        let (path, headers) = loop {
            let mut buffer = [0u8; 512];

            let read_size = self.stream.read(&mut buffer).await?;
            request_buffer.extend_from_slice(&buffer[..read_size]);

            let mut headers = [httparse::EMPTY_HEADER; 32];
            let mut request = httparse::Request::new(&mut headers);
            match request.parse(&request_buffer) {
                Ok(httparse::Status::Complete(_)) => break (request.path, headers),
                Ok(httparse::Status::Partial) => continue,
                Err(e) => {
                    eprintln!("{} : httparse error : {}", self.peer_addr, e);
                    break (None, headers);
                }
            }
        };

        let (code, upgraded);
        if path.is_some() && path.unwrap().starts_with("/img/") {
            if let Some(image_name) = path.unwrap().get(5..) {
                if let Some(image_content) = self.image_list.get(image_name).cloned() {
                    code = 200;
                    self.write_response(200, "OK", "image/png", &image_content)
                        .await?;
                } else {
                    code = HTTP_404.code;
                    self.write_error(&HTTP_404).await?;
                }
            } else {
                code = HTTP_404.code;
                self.write_error(&HTTP_404).await?;
            }
            upgraded = false;
        } else if path == Some("/websocket") {
            (code, upgraded) = self.handle_upgrade_request(&headers).await?;
        } else {
            enum HttpRoutingResult {
                Ok(&'static str, &'static str),
                Err(&'static HttpError),
            }

            let routing_result = match path {
                Some("/") => HttpRoutingResult::Ok(include_str!("../res/index.html"), "text/html"),
                Some("/index.js") => {
                    HttpRoutingResult::Ok(include_str!("../res/index.js"), "text/javascript")
                }
                Some(_) => HttpRoutingResult::Err(&HTTP_404),
                None => HttpRoutingResult::Err(&HTTP_400),
            };
            code = match routing_result {
                HttpRoutingResult::Ok(text, content_type) => {
                    self.write_text_response(200, "OK", content_type, text)
                        .await?;
                    200
                }
                HttpRoutingResult::Err(e) => {
                    self.write_error(e).await?;
                    e.code
                }
            };
            upgraded = false;
        }

        let user_agent = match headers
            .iter()
            .find(|header| header.name.to_lowercase() == "user-agent")
        {
            Some(h) => std::str::from_utf8(h.value).ok(),
            None => None,
        };
        eprintln!(
            "{} : {:?} {:?} : {}",
            self.peer_addr, path, user_agent, code
        );

        if upgraded {
            self.handle_websocket().await
        } else {
            Ok(())
        }
    }
}

pub async fn accept_connections(
    listener: TcpListener,
    write_streams: WriteStreams,
    tweeps: Tweeps,
    image_list: Arc<ImageList>,
) {
    while let Ok((stream, peer_addr)) = listener.accept().await {
        let write_streams_clone = write_streams.clone();
        let tweeps_clone = tweeps.clone();
        let image_list_clone = image_list.clone();
        task::spawn(async move {
            let connection = HttpConnection::new(
                stream,
                peer_addr,
                write_streams_clone,
                tweeps_clone,
                image_list_clone,
            );
            if let Err(error) = connection.handle_connection().await {
                eprintln!("{} : {}", peer_addr, error);
            }
        });
    }
}

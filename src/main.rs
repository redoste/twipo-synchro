use async_std::net::TcpListener;
use async_std::sync::{Arc, Mutex};
use async_std::task;

use futures::prelude::*;

use std::io::{Error as IoError, ErrorKind};

pub mod game;
pub mod http;
pub mod images;

async fn async_main() -> Result<(), IoError> {
    let listen_address = match std::env::args().nth(1) {
        Some(a) => a,
        None => {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "Expected listen address as first argument",
            ))
        }
    };

    let image_list = match images::read_images_from_stdin().await {
        Ok(i) => i,
        Err(e) => {
            eprintln!("{}", e);
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "Invalid image sent from game",
            ));
        }
    };

    let listener = TcpListener::bind(&listen_address).await?;
    eprintln!("**** Start apprication on {} ****", &listen_address);

    let write_streams: http::WriteStreams = Arc::new(Mutex::new(Vec::new()));
    let tweeps: game::Tweeps = Arc::new(Mutex::new(Vec::new()));

    futures::select!(
        _ = http::accept_connections(listener, write_streams.clone(), tweeps.clone(), Arc::new(image_list)).fuse() => Ok(()),
        e = game::read_stdin(write_streams.clone(), tweeps.clone()).fuse() => e,
    )
}

fn main() -> Result<(), IoError> {
    task::block_on(async_main())
}

use async_std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use async_std::sync::{Arc, Mutex, RwLock};
use async_std::task;

use futures::prelude::*;

use std::io::{Error as IoError, ErrorKind};
use std::str::FromStr;

pub mod game;
pub mod http;
pub mod images;

async fn async_main() -> Result<(), IoError> {
    let listen_address_str = match std::env::args().nth(1) {
        Some(a) => a,
        None => {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "Expected listen address as first argument",
            ))
        }
    };

    let listen_address = match SocketAddr::from_str(&listen_address_str) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", e);
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "Invalid listen address provided",
            ));
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

    // We assume that if the user changed the listen address, they know what they're doing and must
    // be able to find the correct address themselves.
    if listen_address.ip() == IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)) {
        match local_ip_address::list_afinet_netifas() {
            Ok(addrs) => {
                for (_, addr) in addrs.iter() {
                    if let std::net::IpAddr::V4(v4addr) = addr {
                        if !v4addr.is_loopback() {
                            eprintln!("   * http://{}:{}/", v4addr, listen_address.port());
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Unable to get your local IPv4 addresses : {}", e);
            }
        }
    }

    let write_streams: http::WriteStreams = Arc::new(Mutex::new(Vec::new()));
    let tweeps: game::Tweeps = Arc::new(Mutex::new(Vec::new()));
    let date: game::Date = Arc::new(RwLock::new(0));

    futures::select!(
        _ = http::accept_connections(listener,
                                     write_streams.clone(),
                                     tweeps.clone(),
                                     date.clone(),
                                     Arc::new(image_list)).fuse() => Ok(()),
        e = game::read_stdin(write_streams.clone(),
                             tweeps.clone(),
                             date.clone()).fuse() => e,
    )
}

fn main() -> Result<(), IoError> {
    task::block_on(async_main())
}

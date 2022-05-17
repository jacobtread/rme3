use std::io;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

use crate::packet::{read_packet, read_packet_contents};

mod tdf;
mod packet;

const HOST: &str = "127.0.0.1:14219";

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind(HOST).await?;
    println!("Server listening on {0}", HOST);
    loop {
        if let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(async move {
                handle_client(stream, addr).await
            });
        }
    }
}


async fn handle_client(mut stream: TcpStream, addr: SocketAddr) {
    println!("New client connected at address {0}\n", addr);
    'ga: loop {
        let packet = read_packet(&mut stream).await;
        match packet {
            Ok(packet) => {
                println!("{:?}", packet);
                match read_packet_contents(&packet) {
                    Ok(content) => {
                        println!("{:?}", content);
                    }
                    Err(err) => {
                        eprintln!("{:?}", err);
                    }
                }
            }
            Err(err) => {
                eprintln!("{:?}", err);
                break 'ga;
            }
        }
    }
}

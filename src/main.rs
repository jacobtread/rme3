mod tdf;

use std::fmt::format;
use std::io;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;

use tokio::net::{TcpListener, TcpStream};

const HOST: &str = "127.0.0.1:14219";

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind(HOST).await?;
    println!("Server listening on {0}", HOST);
    loop {
        if let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(async move {
                println!("New client connected at address {0}\n", addr);
                handle_client(stream, addr)
            });
        }
    }
}

async fn handle_client(mut stream: TcpStream, addr: SocketAddr) {


}

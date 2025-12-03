// WebSocket test server
// Run with: cargo run --bin ws_test_server

use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

async fn handle_connection(stream: TcpStream, addr: SocketAddr) {
    println!("New WebSocket connection from: {}", addr);

    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Error during WebSocket handshake: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("Received: {}", text);

                // Echo back with prefix
                let response = format!("Echo: {}", text);
                if let Err(e) = write.send(Message::Text(response)).await {
                    eprintln!("Error sending message: {}", e);
                    break;
                }
            }
            Ok(Message::Binary(bin)) => {
                println!("Received binary: {} bytes", bin.len());
                if let Err(e) = write.send(Message::Binary(bin)).await {
                    eprintln!("Error sending message: {}", e);
                    break;
                }
            }
            Ok(Message::Ping(ping)) => {
                if let Err(e) = write.send(Message::Pong(ping)).await {
                    eprintln!("Error sending pong: {}", e);
                    break;
                }
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Close(_)) => {
                println!("Client closed connection");
                break;
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    println!("Connection closed: {}", addr);
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:3001";
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");

    println!("WebSocket test server listening on ws://{}", addr);
    println!("\nBehavior:");
    println!("  - Echoes all text messages with 'Echo: ' prefix");
    println!("  - Echoes all binary messages as-is");
    println!("  - Responds to ping/pong");

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr));
    }
}

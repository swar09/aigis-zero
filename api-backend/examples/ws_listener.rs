//! Standalone example demonstrating WebSocket connection and subscription to live telemetry feeds.
//!
//! # Running
//!
//! ```bash
//! cargo run --example ws_listener
//! ```

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ws_url = "ws://127.0.0.1:8080/api/v1/ws?topics=alerts,logs";
    println!("Connecting to Aigis-Zero Live Feed at: {ws_url}");

    let (ws_stream, response) = match connect_async(ws_url).await {
        Ok(res) => res,
        Err(e) => {
            println!("Failed to connect to API server at {ws_url}: {e}");
            println!("Make sure the backend is running: cargo run --bin edr-api-backend");
            return Ok(());
        }
    };

    println!(
        "Connected successfully! (HTTP Status: {})",
        response.status()
    );
    let (mut write, mut read) = ws_stream.split();

    // Send a keepalive ping
    let ping_msg = serde_json::json!({ "action": "ping" });
    write
        .send(Message::Text(ping_msg.to_string().into()))
        .await?;
    println!("Sent ping frame. Listening for live events (Press Ctrl+C to exit)...");

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("[LIVE EVENT RECEIVED]:\n{text}\n");
            }
            Ok(Message::Close(_)) => {
                println!("Server closed WebSocket connection.");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                println!("Error reading from WebSocket: {e}");
                break;
            }
        }
    }

    Ok(())
}

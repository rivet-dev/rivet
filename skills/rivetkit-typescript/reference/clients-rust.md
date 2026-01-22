# Rust

> Source: `src/content/docs/clients/rust.mdx`
> Canonical URL: https://rivet.gg/docs/clients/rust
> Description: The Rivet Rust client provides a way to connect to and interact with actors from Rust applications.

---
## Quickstart

  
### Installation

    Add RivetKit client to your `Cargo.toml`:
    
    ```toml
    [dependencies]
    rivetkit-client = "0.1.0"
    ```
  

  
### Connect to your Actors

    
      Make sure you have a running Rivet actor server to connect to. You can follow the [Node.js & Bun Quickstart](https://rivet.dev/docs/actors/quickstart/backend/) to set up a simple actor server.
    

    ```rust src/main.rs
    use rivetkit_client::{Client, EncodingKind, GetOrCreateOptions, TransportKind};
    use serde_json::json;

    #[tokio::main]
    async fn main() -> anyhow::Result<()> {
        // Create a client connected to your RivetKit manager
        let client = Client::new(
            "http://localhost:8080",
            TransportKind::Sse,
            EncodingKind::Json
        );

        // Connect to a chat room actor
        let chat_room = client.get_or_create(
            "chat-room",
            ["keys-here"].into(),
            GetOrCreateOptions::default()
        )?.connect();
        
        // Listen for new messages
        chat_room.on_event("newMessage", |args| {
            let username = args[0].as_str().unwrap();
            let message = args[1].as_str().unwrap();
            println!("Message from {}: {}", username, message);
        }).await;

        // Send message to room
        chat_room.action("sendMessage", vec![
            json!("william"),
            json!("All the world's a stage.")
        ]).await?;

        // When finished
        client.disconnect();

        Ok(())
    }
    ```
  

  
### Run your client

    In a separate terminal, run your client code:
    
    ```sh
    cargo run
    ```

    Run it again to see the state update.
  

## API Reference

For detailed API documentation, please refer to the [RivetKit Rust client implementation](https://github.com/rivet-dev/rivet/tree/main/rivetkit-rust/packages/client).

_Source doc path: /docs/clients/rust_

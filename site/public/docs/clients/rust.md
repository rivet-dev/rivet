# Rust

The Rivet Rust client provides a way to connect to and interact with actors from Rust applications.

## Quickstart

    Create a new Rust project:
    
    ```sh
    cargo new my-app
    cd my-app
    ```

    Add Rivet client & related dependencies to your project:
    
    ```sh
    cargo add rivetkit-client
    cargo add serde_json
    cargo add tokio --features full
    ```

    Modify `src/main.rs` to connect to your actor:

    ```rust src/main.rs
    use rivetkit_client::;
    use serde_json::json;
    use std::time::Duration;

    #[tokio::main]
    async fn main() -> Result> ", count);
        }).await;
        
        // Call an action
        let result = counter.action("increment", vec![json!(5)]).await?;
        println!("Action: ", result);
        
        // Wait to receive events
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        Ok(())
    }
    ```

    In a separate terminal, run your client code:
    
    ```sh
    cargo run
    ```
    
    You should see output like:
    ```
    Event: 5
    Action: 5
    ```

    Run it again to see the state update.
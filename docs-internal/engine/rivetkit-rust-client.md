# RivetKit Rust Client

The Rust client intentionally uses normal Rust cancellation instead of a
TypeScript-style `AbortSignal`. Futures are cancelled by dropping them, and
`tokio::select!` is the usual way to race actor work against a shutdown signal.

## Cancel a pending action

Dropping the action future cancels the client-side wait. Use `tokio::select!`
when an actor action should stop waiting after a timeout or shutdown signal.

```rust
use std::time::Duration;

use anyhow::Result;
use rivetkit_client::{Client, ClientConfig, GetOptions};
use serde_json::json;

async fn call_with_timeout(client: Client) -> Result<()> {
	let actor = client.get("worker", vec!["a".to_string()], GetOptions::default())?;

	tokio::select! {
		result = actor.action("build", vec![json!({ "id": 1 })]) => {
			let output = result?;
			tracing::info!(?output, "actor action completed");
		}
		_ = tokio::time::sleep(Duration::from_secs(5)) => {
			tracing::warn!("actor action timed out");
		}
	}

	Ok(())
}
```

## Close an actor connection

`ActorConnection::disconnect()` is the explicit close path. Dropping the
connection handle also tears down the client-side websocket ownership; use
`disconnect()` when the peer needs an orderly close before the current scope
ends.

```rust
use anyhow::Result;
use rivetkit_client::{Client, ClientConfig, GetOptions};

async fn connect_then_close() -> Result<()> {
	let client = Client::new(ClientConfig::new("http://127.0.0.1:6420"));
	let actor = client.get("chat", vec!["room-1".to_string()], GetOptions::default())?;
	let conn = actor.connect();

	conn.disconnect().await?;
	Ok(())
}
```

## Thread explicit cancellation

Use `tokio_util::sync::CancellationToken` when multiple tasks should stop
together. Clone the token into each task and race it with the pending client
operation.

```rust
use anyhow::Result;
use rivetkit_client::{Client, GetOptions};
use serde_json::json;
use tokio_util::sync::CancellationToken;

async fn call_until_cancelled(client: Client, cancel: CancellationToken) -> Result<()> {
	let actor = client.get("worker", vec!["b".to_string()], GetOptions::default())?;
	let child = cancel.child_token();

	tokio::select! {
		result = actor.action("run", vec![json!({ "job": "compact" })]) => {
			result?;
		}
		_ = child.cancelled() => {
			tracing::debug!("actor action cancelled by caller");
		}
	}

	Ok(())
}
```

Inside Rust actors, `Ctx<A>::client()` builds the same client type from the
actor's configured envoy endpoint, token, namespace, and pool, then caches it
for the actor context. Use it for actor-to-actor actions, queue sends, raw
HTTP, and websocket connections.

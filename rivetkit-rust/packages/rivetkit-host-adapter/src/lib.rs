//! Binding-independent host logic for RivetKit actors.
//!
//! `rivetkit-core` runs actors and hands each one a stream of [`ActorEvent`]s.
//! Something has to turn those events into calls to user code in another
//! language: run the lifecycle hooks in the right order, bound every call with a
//! timeout, deliver actions, connections, HTTP requests and WebSockets, and shut
//! down cleanly. That logic lives inside each binding (napi, wasm). This crate is
//! meant to hold it once, behind the [`HostActor`] / [`HostInstance`] traits, so a
//! new language binding only maps calls to its own runtime. Today only the Java SDK
//! (through `rivetkit-ffi`) uses it; napi and wasm still carry their own copies, and
//! moving them here is follow-up work.
//!
//! Two layers:
//! - [`event_loop`]: the per-actor loop, generic over [`HostInstance`]. It works
//!   with core types (`ConnHandle`, `Request`, `WebSocket`) and has no opinion on
//!   how bytes reach the host.
//! - [`wire`] (feature `wire`): a byte protocol for C ABI bindings. Events and
//!   calls are a `u32` kind, a few `u64` ids, a small CBOR header, and an untouched
//!   body (user CBOR, HTTP or WebSocket bytes). It also owns the handle table,
//!   leased buffers, and stream credit. `rivetkit-ffi` exposes it as C.
//!
//! Ordering contract: the synchronous part of every [`HostInstance`] method runs
//! in event arrival order, on the actor's event loop task. A host that records the
//! event there (as the wire host does) can keep arrival order without Rust
//! serializing execution. The returned future then runs concurrently.
//!
//! [`ActorEvent`]: rivetkit_core::ActorEvent

pub mod config;
pub mod errors;
pub mod event_loop;
pub mod host;
pub mod http;

#[cfg(feature = "wire")]
pub mod wire;

pub use config::{AdapterConfig, Hooks};
pub use event_loop::actor_factory;
pub use host::{ActionCall, HostActor, HostFuture, HostInstance, HttpReply};
pub use http::ResponseBody;

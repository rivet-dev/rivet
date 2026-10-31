//! Peer channels: every unary request to a remote worker shares one long-lived HTTP stream with
//! the other requests to that worker, instead of opening a stream each.

pub(crate) mod client;
mod frame;
pub(in crate::pubsub) mod server;
pub(crate) mod state;

/// The route of a peer channel's long-lived duplex stream.
pub(crate) const PATH: &str = "/ups/v1/channel";

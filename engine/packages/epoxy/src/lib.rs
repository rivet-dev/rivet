//! Epoxy consensus backed by Depot partition KV.

extern crate self as epoxy;

pub mod client;
pub mod commands;
pub mod consensus;
pub mod consts;
pub mod depot;
pub mod http;
pub mod key_machine;
pub mod membership;
pub mod metrics;
pub mod ops;
pub mod services;
pub mod transport;
pub mod types;



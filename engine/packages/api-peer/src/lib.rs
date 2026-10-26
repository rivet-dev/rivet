//! Internal APIs served on each worker's API-peer listener. The repair routes require the admin
//! token; every other route is served without authentication.

pub mod filesystem;
pub mod operator;
pub mod repair;
pub mod router;
pub mod volumes;

pub use router::router as create_router;

//! Incremental facade for `tokio` task, timer, and selection APIs. Operations use
//! the injected environment so production and simulation share the same callers.

mod select;
pub mod task;
pub mod time;

pub use crate::{select, select_biased};
#[doc(hidden)]
pub use select::Choice as __Choice;

pub use select::when;

//! Single-shot commit path for the stateless depot conveyer.

mod apply;
mod branch_init;
mod dirty;
mod helpers;
mod publish;
mod stage;
mod truncate;

#[cfg(debug_assertions)]
pub mod test_hooks;

#[cfg(not(debug_assertions))]
mod test_hooks;

pub use dirty::clear_sqlite_cmp_dirty_if_observed_idle;
pub(crate) use stage::clear_abandoned_commit_stage;

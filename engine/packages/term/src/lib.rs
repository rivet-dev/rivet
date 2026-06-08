// Re-exports
pub use console;

// Modules
pub mod error;
pub mod format;
pub mod prompt;
pub mod status;

pub fn terminal() -> console::Term {
	console::Term::stderr()
}

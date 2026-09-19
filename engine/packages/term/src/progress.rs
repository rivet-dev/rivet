use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

pub use indicatif;

/// Creates a progress bar drawn to stderr, styled like the `status` helpers.
///
/// The bar is hidden when stderr is not a terminal so piped or captured output stays clean. Call
/// `finish_and_clear` before printing results to stdout.
pub fn bar(msg: impl Into<String>, total: u64) -> ProgressBar {
	let bar = if crate::terminal().is_term() {
		ProgressBar::with_draw_target(Some(total), ProgressDrawTarget::stderr_with_hz(10))
	} else {
		ProgressBar::hidden()
	};

	bar.set_style(
		ProgressStyle::with_template("{prefix:.green.bold} [{bar:30}] {pos}/{len} {msg}")
			.unwrap_or_else(|_| ProgressStyle::default_bar())
			.progress_chars("=> "),
	);
	bar.set_prefix(msg.into());

	bar
}

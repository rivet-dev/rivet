use console::{style, StyledObject};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::{fmt, io, time::Duration};

use crate::util::task;

pub fn link(msg: impl ToString) -> StyledObject<String> {
	style(msg.to_string()).italic().underlined()
}

#[derive(Clone)]
pub enum EitherProgressBar {
	Single(ProgressBar),
	Multi(MultiProgress),
}

pub fn multi_progress_bar(_task: task::TaskCtx) -> MultiProgress {
	let pb = MultiProgress::new();
	// pb.set_draw_target(get_pb_draw_target(task));
	pb
}

pub fn progress_bar(_task: task::TaskCtx) -> ProgressBar {
	// Don't draw the first iteration until the pb is styled
	let pb = ProgressBar::hidden();
	pb.set_style(pb_style_file(false));
	// pb.set_draw_target(get_pb_draw_target(task));
	pb.enable_steady_tick(Duration::from_millis(1000));
	pb
}

pub fn pb_style_file(include_spinner: bool) -> ProgressStyle {
	ProgressStyle::default_bar()
		.progress_chars("=> ")
		.template(&if include_spinner {
            format!(
                "{{spinner:.dim}} {}{{eta:.dim}}{} [{{bar:23}}] {{percent:.bold}}{} {}{{bytes:.dim}}{}{{total_bytes:.dim}}{} {{binary_bytes_per_sec:.dim}}{} {{wide_msg}}",
                style("(T-").dim(),
                style(")").dim(),
                style("%").bold(),
                style("(").dim(),
                style("/").dim(),
                style(",").dim(),
                style(")").dim(),
            )
        } else {
            format!(
                "{}{{eta:3.dim}} [{{bar:12}}] {{percent:.bold}}{} {}{{bytes:.dim}}{}{{total_bytes:.dim}}{} {{binary_bytes_per_sec:.dim}}{} {{wide_msg}}",
                style("T-").dim(),
                style("%").bold(),
                style("(").dim(),
                style("/").dim(),
                style(",").dim(),
                style(")").dim(),
            )
        })
		.expect("invalid progress bar style")
}

pub fn pb_style_error() -> ProgressStyle {
	ProgressStyle::default_bar()
		.template(&format!("{} {{wide_msg:.red}}", style("!").bold().red()))
		.expect("invalid progress bar style")
}

pub fn get_pb_draw_target(task: task::TaskCtx) -> ProgressDrawTarget {
	indicatif::ProgressDrawTarget::term_like(Box::new(TaskDrawTarget::new(task)))
}

pub struct TaskDrawTarget {
	task: task::TaskCtx,
}

impl fmt::Debug for TaskDrawTarget {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("TaskDrawTarget").finish_non_exhaustive()
	}
}

impl TaskDrawTarget {
	pub fn new(task: task::TaskCtx) -> TaskDrawTarget {
		TaskDrawTarget { task }
	}
}

impl indicatif::TermLike for TaskDrawTarget {
	fn width(&self) -> u16 {
		80
	}

	fn move_cursor_up(&self, _n: usize) -> io::Result<()> {
		Ok(())
	}

	fn move_cursor_down(&self, _n: usize) -> io::Result<()> {
		Ok(())
	}

	fn move_cursor_right(&self, _n: usize) -> io::Result<()> {
		Ok(())
	}

	fn move_cursor_left(&self, _n: usize) -> io::Result<()> {
		Ok(())
	}

	fn write_line(&self, s: &str) -> io::Result<()> {
		let s = s.trim();
		if !s.is_empty() {
			self.task.log(s);
		}
		Ok(())
	}

	fn write_str(&self, s: &str) -> io::Result<()> {
		let s = s.trim();
		if !s.is_empty() {
			self.task.log(s);
		}
		Ok(())
	}

	fn clear_line(&self) -> io::Result<()> {
		Ok(())
	}

	fn flush(&self) -> io::Result<()> {
		Ok(())
	}
}

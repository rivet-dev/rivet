use console::style;
use std::fmt::Display;

pub fn info(msg: impl Display, data: impl Display) {
	eprintln!("{} {}", style(msg).bold().blue(), data);
}

pub fn progress(msg: impl Display, data: impl Display) {
	eprintln!("{} {}", style(msg).bold().green(), data);
}

pub fn success(msg: impl Display, data: impl Display) {
	eprintln!("{} {}", style(msg).bold().green(), data);
}

pub fn warn(msg: impl Display, data: impl Display) {
	eprintln!("{} {}", style(msg).bold().yellow(), data);
}

pub fn error(msg: impl Display, data: impl Display) {
	eprintln!("{} {}", style(msg).bold().red(), data);
}

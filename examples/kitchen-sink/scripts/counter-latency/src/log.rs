// ANSI helpers, gradient color, log formatting helpers. 1:1 port of the
// console.log layer in scripts/counter-latency.ts.

use chrono::Utc;

pub const RESET: &str = "\x1b[0m";
pub const GREEN: &str = "\x1b[38;2;0;255;0m";
pub const RED: &str = "\x1b[38;2;255;0;0m";
pub const YELLOW: &str = "\x1b[38;2;255;200;0m";
pub const BLUE: &str = "\x1b[38;2;80;160;255m";
pub const CYAN: &str = "\x1b[38;2;0;200;220m";
pub const DIM: &str = "\x1b[2m";
pub const BOLD: &str = "\x1b[1m";

pub const COLOR_MIN_MS: f64 = 800.0;
pub const COLOR_MAX_MS: f64 = 2_000.0;

pub fn gradient_color(ms: f64) -> String {
	let clamped = ms.clamp(COLOR_MIN_MS, COLOR_MAX_MS);
	let t = (clamped - COLOR_MIN_MS) / (COLOR_MAX_MS - COLOR_MIN_MS);
	let r;
	let g;
	if t <= 0.5 {
		r = (t * 2.0 * 255.0).round() as u32;
		g = 255u32;
	} else {
		r = 255u32;
		g = ((1.0 - (t - 0.5) * 2.0) * 255.0).round() as u32;
	}
	format!("\x1b[38;2;{};{};0m", r, g)
}

pub fn color_ms(ms: f64) -> String {
	let fixed = format!("{:>5}", ms.round() as i64);
	format!("{}{}ms{}", gradient_color(ms), fixed, RESET)
}

pub fn pad(s: &str, n: usize) -> String {
	if s.len() >= n {
		s.to_string()
	} else {
		let mut buf = String::with_capacity(n);
		buf.push_str(s);
		for _ in s.len()..n {
			buf.push(' ');
		}
		buf
	}
}

pub fn format_actor(actor_id: Option<&str>) -> String {
	match actor_id {
		Some(id) if !id.is_empty() => format!(" actor={}", id),
		_ => String::new(),
	}
}

pub fn iso_now() -> String {
	Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

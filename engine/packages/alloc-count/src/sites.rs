//! Sampled allocation sites: every Nth allocation in the process records its call stack, and
//! [`report`] groups the samples by the innermost frames outside the allocator and the standard
//! library. Sampling allocates and walks stacks, so counts taken while it runs are not
//! representative; measure counts and sites in separate runs.

use std::{
	cell::Cell,
	collections::HashMap,
	fmt::Write,
	sync::{
		Mutex,
		atomic::{AtomicU64, Ordering},
	},
};

/// Frames kept per sample.
const DEPTH: usize = 40;

/// Sample one allocation in this many; zero turns sampling off.
static EVERY: AtomicU64 = AtomicU64::new(0);

struct Sample {
	frames: [usize; DEPTH],
	len: usize,
	bytes: u64,
}

/// The interval of a paused sampling run, for scaling its report.
static PAUSED: AtomicU64 = AtomicU64::new(0);

static SAMPLES: Mutex<Vec<Sample>> = Mutex::new(Vec::new());

thread_local! {
	static SAMPLING: Cell<bool> = const { Cell::new(false) };
}

/// Starts sampling one allocation in `every`, discarding earlier samples.
pub fn start(every: u64) {
	PAUSED.store(every, Ordering::Relaxed);
	SAMPLES
		.lock()
		.unwrap_or_else(|poison| poison.into_inner())
		.clear();
	EVERY.store(every, Ordering::Relaxed);
}

/// Stops sampling, keeping the samples for [`report`].
pub fn stop() {
	EVERY.store(0, Ordering::Relaxed);
}

/// Pauses sampling like [`stop`], remembering the interval so [`report`] still scales by it.
pub fn pause() {
	PAUSED.store(
		EVERY
			.swap(0, Ordering::Relaxed)
			.max(PAUSED.load(Ordering::Relaxed)),
		Ordering::Relaxed,
	);
}

/// Resumes sampling one allocation in `every`, keeping earlier samples.
pub fn resume(every: u64) {
	PAUSED.store(every, Ordering::Relaxed);
	EVERY.store(every, Ordering::Relaxed);
}

/// Called by the allocator with the slot's allocation sequence number.
pub(crate) fn maybe_sample(sequence: u64, size: usize) {
	let every = EVERY.load(Ordering::Relaxed);
	if every == 0 || sequence % every != 0 {
		return;
	}
	let Ok(false) = SAMPLING.try_with(|sampling| sampling.replace(true)) else {
		return;
	};
	let mut sample = Sample {
		frames: [0; DEPTH],
		len: 0,
		bytes: size as u64,
	};
	// SAFETY: the reentrancy flag keeps this thread from walking its stack twice at once, and
	// the walk only reads instruction pointers.
	unsafe {
		backtrace::trace_unsynchronized(|frame| {
			sample.frames[sample.len] = frame.ip() as usize;
			sample.len += 1;
			sample.len < DEPTH
		});
	}
	SAMPLES
		.lock()
		.unwrap_or_else(|poison| poison.into_inner())
		.push(sample);
	let _ = SAMPLING.try_with(|sampling| sampling.set(false));
}

fn is_noise(name: &str) -> bool {
	const NOISE: [&str; 18] = [
		"rivet_alloc_count",
		"backtrace::",
		"__rust",
		"__rdl",
		"alloc::",
		"core::",
		"std::",
		"<alloc::",
		"<core::",
		"<std::",
		"hashbrown::",
		"<hashbrown::",
		"bytes::",
		"<bytes::",
		"smallvec::",
		"<smallvec::",
		"serde_bare::",
		"<serde_bare::",
	];
	NOISE.iter().any(|prefix| name.starts_with(prefix))
}

/// Groups the samples by their first `frames` symbol names outside the allocator and the
/// standard library, with source lines when `ALLOC_SITES_LINES` is set, and returns the `top`
/// groups with the most samples, scaled back up to estimated allocations and bytes.
pub fn report(top: usize, frames: usize) -> String {
	let every = EVERY
		.load(Ordering::Relaxed)
		.max(PAUSED.load(Ordering::Relaxed));
	// `ALLOC_SITES_LINES` adds each frame's source line, which splits inlined frames apart.
	let lines = std::env::var_os("ALLOC_SITES_LINES").is_some();
	let _ = SAMPLING.try_with(|sampling| sampling.set(true));
	let samples = std::mem::take(&mut *SAMPLES.lock().unwrap_or_else(|poison| poison.into_inner()));
	let mut names = HashMap::<usize, String>::new();
	let mut groups = HashMap::<Vec<String>, (u64, u64)>::new();
	for sample in &samples {
		let mut key = Vec::new();
		for &ip in &sample.frames[..sample.len] {
			let name = names.entry(ip).or_insert_with(|| {
				let mut name = String::from("?");
				backtrace::resolve(ip as *mut std::ffi::c_void, |symbol| {
					if let Some(symbol_name) = symbol.name() {
						let mut full = format!("{symbol_name:#}");
						if lines
							&& let (Some(file), Some(line)) = (symbol.filename(), symbol.lineno())
						{
							let file = file.to_string_lossy();
							let file = file.rsplit("/packages/").next().unwrap_or(&file);
							let _ = write!(full, " @ {file}:{line}");
						}
						if name == "?" || !is_noise(&full) {
							name = full;
						}
					}
				});
				name
			});
			if is_noise(name) || name == "?" {
				continue;
			}
			key.push(name.clone());
			if key.len() == frames {
				break;
			}
		}
		let group = groups.entry(key).or_default();
		group.0 += 1;
		group.1 += sample.bytes;
	}
	let mut groups: Vec<_> = groups.into_iter().collect();
	groups.sort_by(|a, b| b.1.0.cmp(&a.1.0));
	let scale = every.max(1);
	let mut out = String::new();
	let _ = writeln!(
		out,
		"{} samples, one per {scale} allocations",
		samples.len()
	);
	for (stack, (count, bytes)) in groups.into_iter().take(top) {
		let _ = writeln!(
			out,
			"{:>6.1}% {:>10} allocs {:>12} bytes",
			100.0 * count as f64 / samples.len().max(1) as f64,
			count * scale,
			bytes * scale
		);
		for frame in stack {
			let _ = writeln!(out, "         {frame}");
		}
	}
	let _ = SAMPLING.try_with(|sampling| sampling.set(false));
	out
}

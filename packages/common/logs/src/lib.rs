#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::Logs;
#[cfg(windows)]
pub use windows::Logs;

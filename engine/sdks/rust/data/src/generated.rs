include!(concat!(env!("OUT_DIR"), "/combined_imports.rs"));

/// `From` impls between consecutive schema versions for the types that did not change. See
/// `converters` in `build.rs`.
mod converters {
	include!(concat!(env!("OUT_DIR"), "/converters.rs"));
}

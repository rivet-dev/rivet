use prometheus::*;

lazy_static::lazy_static! {
	/// The process registry. On Linux it also carries the process collector, so every
	/// process that exports this registry reports its own RSS, CPU time and open files.
	pub static ref REGISTRY: Registry = {
		let registry = Registry::new_custom(Some("rivet".to_string()), Some(labels! {})).unwrap();
		#[cfg(target_os = "linux")]
		registry
			.register(Box::new(process_collector::ProcessCollector::for_self()))
			.unwrap();
		registry
	};
}

use rivet_metrics::prometheus::{Encoder, TextEncoder};

pub(crate) fn render_global_metrics() -> String {
	let encoder = TextEncoder::new();
	let metric_families = rivet_metrics::REGISTRY.gather();
	let mut encoded = Vec::new();
	encoder
		.encode(&metric_families, &mut encoded)
		.expect("encode metrics");
	String::from_utf8(encoded).expect("metrics should be utf-8")
}

pub(crate) fn metric_line_for_actor(line: &str, name: &str, actor_id_gen: &str) -> bool {
	line.starts_with(name) && line.contains(&format!("actor_id_gen=\"{actor_id_gen}\""))
}

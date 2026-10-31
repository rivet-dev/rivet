use rivet_config::DynamicConfigUpdate;

/// Apply tracing overrides from the durable cluster config.
pub(crate) fn apply_tracing(update: &DynamicConfigUpdate) {
	match &update.tracing_filter {
		Some(Some(filter)) => {
			if let Err(err) = rivet_runtime::reload_log_filter(filter) {
				tracing::error!(?err, "failed to reload log filter");
			}
		}
		Some(None) => {
			if let Err(err) = rivet_runtime::reload_log_filter("") {
				tracing::error!(?err, "failed to reload log filter to default");
			}
		}
		None => {}
	}

	match update.tracing_sampler_ratio {
		Some(Some(ratio)) => {
			if let Err(err) = rivet_metrics_server::set_sampler_ratio(ratio) {
				tracing::error!(?err, "failed to reload sampler ratio");
			}
		}
		Some(None) => {
			if let Err(err) = rivet_metrics_server::set_sampler_ratio(0.001) {
				tracing::error!(?err, "failed to reload sampler ratio to default");
			}
		}
		None => {}
	}
}

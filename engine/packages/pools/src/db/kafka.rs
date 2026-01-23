use anyhow::Result;
use rivet_config::Config;

#[tracing::instrument(skip(config))]
pub fn setup(config: &Config) -> Result<Option<rdkafka::producer::FutureProducer>> {
	if let Some(kafka) = config.kafka() {
		tracing::debug!("kafka connecting");

		let producer = rdkafka::ClientConfig::new()
			.set("bootstrap.servers", &kafka.url.to_string())
			.set("security.protocol", "sasl_ssl")
			.set("sasl.mechanism", "SCRAM-SHA-256")
			.set("sasl.username", &kafka.username)
			.set("sasl.password", kafka.password.read())
			.set("ssl.ca.pem", kafka.ca_pem.read())
			.create::<rdkafka::producer::FutureProducer>()?;

		Ok(Some(producer))
	} else {
		Ok(None)
	}
}

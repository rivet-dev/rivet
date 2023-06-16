// TODO: Remove completely, only used for RPC
pub trait Endpoint {
	type Request: prost::Message;
	type Response: prost::Message + Default;

	const NAME: &'static str;
	const TIMEOUT: std::time::Duration;
}

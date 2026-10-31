use rivet_error::RivetError;

#[derive(Debug, RivetError)]
#[error("ups", "no_responders", "The worker has no consumer for this subject.")]
pub struct NoResponders;

#[derive(Debug, RivetError)]
#[error("ups", "unknown_worker", "The worker is not in current membership.")]
pub struct UnknownWorker;

#[derive(Debug, RivetError)]
#[error("ups", "wrong_worker", "The target worker incarnation has changed.")]
pub struct WrongWorker;

#[derive(Debug, RivetError)]
#[error(
	"ups",
	"timeout",
	"The operation deadline expired; it may still have completed."
)]
pub struct Timeout;

#[derive(Debug, RivetError)]
#[error(
	"ups",
	"overloaded",
	"UPS capacity is full; the operation was not admitted."
)]
pub struct Overloaded;

/// Topology could not resolve the endpoint, or the HTTP adapter proved connection failure
/// before any request was sent. Nothing ran, so the caller may retry.
#[derive(Debug, RivetError)]
#[error(
	"ups",
	"endpoint_unresolved",
	"The worker's endpoint could not be resolved; the request was not sent."
)]
pub struct EndpointUnresolved;

#[derive(Debug, RivetError)]
#[error(
	"ups",
	"consumer_closed",
	"The consumer closed without replying; the operation may have executed."
)]
pub struct ConsumerClosed;

#[derive(Debug, RivetError)]
#[error(
	"ups",
	"subscription_exists",
	"This worker already has a consumer for the subject."
)]
pub struct SubscriptionExists;

#[derive(Debug, RivetError)]
#[error(
	"ups",
	"invalid_message",
	"The message exceeds the limit or has an invalid subject."
)]
pub struct InvalidMessage;

#[derive(Debug, RivetError)]
#[error(
	"ups",
	"transport",
	"Worker transport failed; the operation may have executed."
)]
pub struct Transport;

#[derive(Debug, RivetError)]
#[error(
	"ups",
	"unexpected_packet",
	"Expected exactly one packet followed by the end of the stream."
)]
pub struct UnexpectedPacket;

#[derive(Debug, RivetError)]
#[error(
	"ups",
	"invalid_stream",
	"The packet stream is malformed or ended without a terminal frame."
)]
pub struct InvalidStream;

/// The caller's authority ended before local or remote dispatch; UPS did not submit this request.
#[derive(Debug, RivetError)]
#[error(
	"ups",
	"dispatch_rejected",
	"The caller revoked the request before dispatch."
)]
pub struct DispatchRejected;

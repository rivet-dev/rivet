/// Returns whether the given protocol version needs tunnel ack messages.
///
/// Older protocols have GC cycles to check for tunnel ack, so we need to send DeprecatedTunnelAck
/// for backwards compatibility.
pub fn version_needs_tunnel_ack(version: u16) -> bool {
	version <= 2
}

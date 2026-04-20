use crate::actor::connection::ConnHandle;

#[derive(Clone, Debug, Default)]
pub struct EventBroadcaster;

impl EventBroadcaster {
	pub fn broadcast<I>(&self, connections: I, name: &str, args: &[u8])
	where
		I: IntoIterator<Item = ConnHandle>,
	{
		for connection in connections {
			if connection.is_subscribed(name) {
				connection.send(name, args);
			}
		}
	}
}

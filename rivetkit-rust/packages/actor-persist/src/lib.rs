pub mod generated;
pub mod versioned;

// Re-export latest.
pub use generated::v4::*;

pub const CURRENT_VERSION: u16 = 4;

impl Default for generated::v4::Actor {
	fn default() -> Self {
		Self {
			input: None,
			has_initialized: false,
			state: Vec::new(),
			scheduled_events: Vec::new(),
		}
	}
}

impl Default for generated::v4::ScheduleEvent {
	fn default() -> Self {
		Self {
			event_id: String::new(),
			timestamp: 0,
			action: String::new(),
			args: None,
		}
	}
}

impl Default for generated::v4::Subscription {
	fn default() -> Self {
		Self {
			event_name: String::new(),
		}
	}
}

impl Default for generated::v4::Conn {
	fn default() -> Self {
		Self {
			id: String::new(),
			parameters: Vec::new(),
			state: Vec::new(),
			subscriptions: Vec::new(),
			gateway_id: [0; 4],
			request_id: [0; 4],
			server_message_index: 0,
			client_message_index: 0,
			request_path: String::new(),
			request_headers: std::collections::HashMap::new(),
		}
	}
}

impl Default for generated::v4::QueueMetadata {
	fn default() -> Self {
		Self {
			next_id: 0,
			size: 0,
		}
	}
}

impl Default for generated::v4::QueueMessage {
	fn default() -> Self {
		Self {
			name: String::new(),
			body: Vec::new(),
			created_at: 0,
			failure_count: None,
			available_at: None,
			in_flight: None,
			in_flight_at: None,
		}
	}
}

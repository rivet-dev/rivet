use rivet_util as util;
use uuid::Uuid;

pub const THREAD_TAIL_MESSAGE_EXPIRE_DURATION: i64 = util::duration::days(3) / 1000; // Seconds
pub const USER_THREAD_HISTORY_EXPIRE_DURATION: i64 = THREAD_TAIL_MESSAGE_EXPIRE_DURATION; // Seconds
pub const TYPING_STATUS_EXPIRE_DURATION: i64 = util::duration::seconds(30) / 1000; // Seconds

/// ZSET<thread id, send ts>
///
/// Threads that the user is a part of sorted by the last message timestamp.
/// Powers the recent threads sidebar.
pub fn user_thread_history(user_id: Uuid) -> String {
	format!("{{global}}:chat:user:{}:thread_history", user_id)
}

/// BOOL
///
/// Indicates that the user thread history is loaded in to Redis. We can't use
/// `user_thread_history` since it may be empty.
pub fn user_thread_history_loaded(user_id: Uuid) -> String {
	format!("{{global}}:chat:user:{}:thread_history:loaded", user_id)
}

/// HASH
pub fn thread_tail_message(thread_id: Uuid) -> String {
	format!("{{global}}:chat:thread:{}:tail_message", thread_id)
}

pub mod thread_tail_message {
	pub const MESSAGE_ID: &str = "m";
	pub const SEND_TS: &str = "st";
	pub const MESSAGE_BUF: &str = "mb";
}

/// HMAP<user id, rivet.backend.chat.TypingStatus>
pub fn typing_statuses(thread_id: Uuid) -> String {
	format!("{{global}}:chat:thread:{}:typing_statuses", thread_id)
}

/// ZSET<user id, update ts>
pub fn typing_statuses_update_ts(thread_id: Uuid) -> String {
	format!("{{global}}:chat:thread:{}:typing_statuses_update_ts", thread_id)
}

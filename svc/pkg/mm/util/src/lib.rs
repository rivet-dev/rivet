pub mod consts;
pub mod defaults;
pub mod key;
pub mod test;
pub mod verification;
pub mod version_migrations;

pub enum JoinKind {
	Normal,
	Party,
	Direct,
}

impl JoinKind {
	pub fn short(self) -> &'static str {
		match self {
			JoinKind::Normal => "normal",
			JoinKind::Party => "party",
			JoinKind::Direct => "direct",
		}
	}
}

#[derive(Debug, PartialEq, strum::FromRepr)]
#[repr(u8)]
pub enum FindQueryStatus {
	/// Lobby is creating or in between mm-lobby-find and
	/// mm-lobby-find-try-complete.
	Pending = 0,
	/// Find finished and lobby is ready.
	Complete = 1,
	/// There was an error.
	Fail = 2,
}

/// Formats the port label to be used in Nomad.
///
/// Prefixing this port ensure that the user defined port names don't interfere
/// with other ports.
pub fn format_nomad_port_label(port_label: &str) -> String {
	format!("game-{port_label}")
}

pub const RUNC_SETUP_CPU: i32 = 50;
pub const RUNC_SETUP_MEMORY: i32 = 32;
pub const RUNC_CLEANUP_CPU: i32 = 50;
pub const RUNC_CLEANUP_MEMORY: i32 = 32;

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Result, bail};

/// Inspector tab declaration carried on `ActorConfig`. Either a custom tab
/// (id + label + source root) or a hide modifier for a built-in tab.
/// Validation of id collisions and source presence happens upstream in the
/// TypeScript Zod schema or the Rust builder; `validate_inspector_tabs`
/// below is the runtime authority that closes the gap when direct Rust
/// callers bypass the upstream layers.
#[derive(Clone, Debug)]
pub enum InspectorTabEntry {
	Custom {
		id: String,
		label: String,
		/// Icon identifier the dashboard maps to a glyph (see the
		/// dashboard's icon registry). `None` falls back to a generic
		/// icon on the dashboard side.
		icon: Option<String>,
		root: PathBuf,
	},
	HideBuiltin {
		id: String,
	},
}

/// Set of built-in inspector tab ids the dashboard ships. The Rust runtime
/// uses this both to reject custom-tab ids that collide with a built-in and
/// to validate `HideBuiltin { id }` entries reference a known tab.
pub const BUILTIN_TAB_IDS: &[&str] = &[
	"workflow",
	"database",
	"state",
	"queue",
	"schedules",
	"connections",
	"console",
];

/// Custom tab id grammar enforced at every layer (TS Zod, NAPI, Rust). Slashes
/// are forbidden because the URL splits `/inspector/custom-tabs/<id>/<rest>`
/// on the first `/`.
fn is_valid_custom_tab_id(id: &str) -> bool {
	!id.is_empty()
		&& id
			.chars()
			.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl InspectorTabEntry {
	pub fn id(&self) -> &str {
		match self {
			Self::Custom { id, .. } | Self::HideBuiltin { id } => id,
		}
	}

	fn validate(&self) -> Result<()> {
		match self {
			Self::Custom {
				id,
				label,
				icon,
				root,
			} => {
				if !is_valid_custom_tab_id(id) {
					bail!(
						"inspector tab id {id:?} is invalid: must be non-empty and contain only [a-zA-Z0-9_-]"
					);
				}
				if BUILTIN_TAB_IDS.contains(&id.as_str()) {
					bail!(
						"inspector tab id {id:?} collides with a built-in tab; use {{ id: {id:?}, hidden: true }} to hide instead"
					);
				}
				if label.is_empty() {
					bail!("inspector tab {id:?} has an empty label");
				}
				if root.as_os_str().is_empty() {
					bail!("inspector tab {id:?} has an empty source path");
				}
				if let Some(icon) = icon
					&& icon.is_empty()
				{
					bail!("inspector tab {id:?} has an empty icon string");
				}
				Ok(())
			}
			Self::HideBuiltin { id } => {
				if !BUILTIN_TAB_IDS.contains(&id.as_str()) {
					bail!(
						"inspector tab hide id {id:?} is not a known built-in (one of {:?})",
						BUILTIN_TAB_IDS
					);
				}
				Ok(())
			}
		}
	}
}

/// Validates a list of `InspectorTabEntry` values: each entry on its own
/// rules plus pairwise duplicate-id rejection. Runtime authority for
/// rejecting malformed configs that bypass the TypeScript Zod layer.
pub fn validate_inspector_tabs(entries: &[InspectorTabEntry]) -> Result<()> {
	let mut seen = HashSet::new();
	for entry in entries {
		entry.validate()?;
		if !seen.insert(entry.id()) {
			bail!("inspector tabs contain duplicate id {:?}", entry.id());
		}
	}
	Ok(())
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/inspector_tabs.rs"]
mod tests;

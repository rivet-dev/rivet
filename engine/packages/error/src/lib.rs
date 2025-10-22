mod error;
mod schema;

pub use error::RivetError;
pub use schema::{MacroMarker, RivetErrorSchema, RivetErrorSchemaWithMeta};

pub use rivet_error_macros::RivetError;

pub static INTERNAL_ERROR: RivetErrorSchema =
	RivetErrorSchema::__internal_new("core", "internal_error", "An internal error occurred");

#[doc(hidden)]
pub use indoc;

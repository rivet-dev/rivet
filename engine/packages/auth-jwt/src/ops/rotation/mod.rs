pub mod request_emergency;
pub mod request_normal;
pub mod status;

pub const WORKFLOW_TAGS: [(&str, &str); 1] = [("auth", "jwt")];

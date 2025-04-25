use chirp_workflow::prelude::GlobalResult;

/// The maximum number of path components allowed in a route path.
///
/// This restricts the depth of paths used in routes, which affects both
/// validation during creation and the number of prefixes checked during routing.
///
/// For example, a path with 8 components would be "/a/b/c/d/e/f/g/h".
pub const MAX_PATH_COMPONENTS: usize = 8;

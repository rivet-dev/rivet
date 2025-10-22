use std::{collections::HashMap, sync::Arc};

pub(crate) mod cursor;
pub mod event;
pub mod location;
pub(crate) mod removed;

pub(crate) type History = Arc<HashMap<location::Location, Vec<event::Event>>>;

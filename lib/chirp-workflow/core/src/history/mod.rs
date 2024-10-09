use std::{collections::HashMap, sync::Arc};

pub mod cursor;
pub mod event;
pub mod location;
pub mod removed;

pub type History = Arc<HashMap<location::Location, Vec<event::Event>>>;

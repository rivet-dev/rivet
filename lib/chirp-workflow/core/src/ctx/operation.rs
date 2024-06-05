use uuid::Uuid;

use crate::DatabaseHandle;

pub struct OperationCtx {
	pub db: DatabaseHandle,
	pub workflow_id: Uuid,
}

impl OperationCtx {
	pub fn new(db: DatabaseHandle, workflow_id: Uuid) -> Self {
		OperationCtx { workflow_id, db }
	}
}

impl OperationCtx {
	// TODO:
}

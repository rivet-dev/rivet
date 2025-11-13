use crate::db::BumpSubSubject;

pub fn convert(subject: BumpSubSubject) -> String {
	match subject {
		BumpSubSubject::Worker => "gasoline.worker.bump".into(),
		BumpSubSubject::WorkflowComplete { workflow_id } => {
			format!("gasoline.workflow.complete.{workflow_id}")
		}
		BumpSubSubject::SignalPublish { to_workflow_id } => {
			format!("gasoline.signal.for-workflow.{to_workflow_id}")
		}
	}
}

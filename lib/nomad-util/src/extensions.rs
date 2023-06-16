use crate::error::NomadError;

pub trait JobExt {
	fn job_id(&self) -> Result<String, NomadError>;
}

impl JobExt for nomad_client::models::Job {
	fn job_id(&self) -> Result<String, NomadError> {
		self.ID.clone().ok_or(NomadError::MissingJobId)
	}
}

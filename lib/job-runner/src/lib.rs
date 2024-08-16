pub mod log_shipper;
pub mod throttle;

pub enum Manager {
	DynamicServers { server_id: String },
	JobRun { run_id: String },
}

mod template {
	use serde::Serialize;

	#[derive(Serialize)]
	pub struct Service {
		pub name: String,
		pub service_kind: String,
		pub runtime_kind: String,
		pub dependencies: Vec<ServiceDependency>,
	}

	#[derive(Serialize)]
	pub struct ServiceDependency {
		pub name: String,
		pub name_screaming_snake: String,
		pub service_kind: String,
		pub runtime_kind: String,
	}

	#[derive(Serialize)]
	pub struct BatchService {
		pub name: String,
		pub service_kind: String,
		pub runtime_kind: String,
	}
}

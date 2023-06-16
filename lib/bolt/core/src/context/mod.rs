mod project;
mod service;

pub use project::*;
pub use service::*;

/// Defines what this code is being ran for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunContext {
	/// Being ran as a standalone binary service.
	Service,
	/// Being ran as a test.
	Test,
}

impl RunContext {
	pub fn short(&self) -> &str {
		match self {
			RunContext::Service => "service",
			RunContext::Test => "test",
		}
	}
}

/// How the service is going to be built.
#[derive(Eq, PartialEq, Clone, Copy)]
pub enum BuildContext {
	Bin { optimization: BuildOptimization },
	Test,
}

impl BuildContext {
	pub fn short(&self) -> String {
		match self {
			BuildContext::Bin { optimization } => format!("bin-{}", optimization.short()),
			BuildContext::Test => "test".into(),
		}
	}

	pub fn path(&self) -> String {
		match self {
			BuildContext::Bin { optimization } => format!("bin/{}", optimization.short()),
			BuildContext::Test => "test".into(),
		}
	}
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum BuildOptimization {
	Release,
	Debug,
}

impl BuildOptimization {
	pub fn short(&self) -> &str {
		match self {
			BuildOptimization::Release => "release",
			BuildOptimization::Debug => "debug",
		}
	}
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum RustBuildTarget {
	Native,
	Musl,
}

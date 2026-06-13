//! The rivet-engine subprocess manager lives in the standalone
//! `rivetkit-engine-process` crate so the CLI and other hosts can reuse the
//! same resolution, spawn, reuse, and orphan-lifetime logic. This module
//! re-exports it for existing in-crate callers.

pub use rivetkit_engine_process::{
	EngineProcessError, EngineProcessManager, EngineResolverConfig, ResolvedEngine, engine_db_path,
	engine_env, resolve_engine_binary, resolve_engine_binary_path,
};

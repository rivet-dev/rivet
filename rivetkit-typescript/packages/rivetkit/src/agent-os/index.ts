// agentOS no longer ships inside RivetKit. This entrypoint remains so existing
// imports fail with migration guidance instead of a missing-module error.
throw new Error(
	"rivetkit/agent-os has been removed. Use @rivet-dev/agentos-core to run agentOS VMs in your process, or hosted agentOS with @rivet-dev/agentos. See https://rivet.dev/agentos/docs",
);

import { docker, sandboxActor } from "rivetkit/sandbox";

export const dockerSandboxActor = sandboxActor({
	provider: docker({
		image: "node:22-bookworm-slim",
		installAgents: [],
	}),
});

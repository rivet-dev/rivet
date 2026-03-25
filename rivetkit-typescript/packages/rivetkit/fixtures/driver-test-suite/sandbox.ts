import { sandboxActor } from "rivetkit/sandbox";
import { docker } from "rivetkit/sandbox/docker";

export const dockerSandboxActor = sandboxActor({
	provider: docker({
		image: "node:22-bookworm-slim",
	}),
});

import { sandboxActor } from "rivetkit/sandbox";
import { e2b } from "rivetkit/sandbox/e2b";

export const e2bSandboxActor = sandboxActor({
	provider: e2b(),
});

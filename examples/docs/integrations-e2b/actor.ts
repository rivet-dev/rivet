import { posix } from "node:path";
import type { Sandbox, SandboxActorContext } from "@rivet-dev/sandbox-adapter";
import { e2bProvider } from "@rivet-dev/sandbox-adapter/e2b";
import { actor, event, setup } from "rivetkit";

// One provider object serves every Actor of this definition.
const provider = e2bProvider({ template: "base" });

interface BuildState {
	sandboxId: string | null;
}

export const build = actor({
	state: { sandboxId: null } as BuildState,
	events: {
		// Command output, as it arrives.
		output: event<string>(),
	},
	actions: {
		run: async (c, command: string) => {
			const sandbox = await openSandbox(c, c.state);
			const decoder = new TextDecoder();
			return sandbox.exec(command, {
				cwd: sandbox.cwd,
				timeoutMs: 60_000,
				// Stops the command if the Actor shuts down.
				signal: c.abortSignal,
				onData: (chunk) => c.broadcast("output", decoder.decode(chunk, { stream: true })),
			});
		},
		saveFile: async (c, path: string, content: string) => {
			const sandbox = await openSandbox(c, c.state);
			const target = posix.resolve(sandbox.cwd, path);
			await sandbox.mkdir(posix.dirname(target));
			await sandbox.writeFile(target, content);
		},
		readFile: async (c, path: string) => {
			const sandbox = await openSandbox(c, c.state);
			const bytes = await sandbox.readFile(posix.resolve(sandbox.cwd, path));
			return new TextDecoder().decode(bytes);
		},
	},
	// The sandbox stops while the Actor sleeps and is removed along with the Actor.
	onSleep: async (c) => {
		if (c.state.sandboxId) await provider.suspend?.(c, c.state.sandboxId);
	},
	onDestroy: async (c) => {
		if (c.state.sandboxId) await provider.destroy?.(c, c.state.sandboxId);
	},
});

// Connects to this Actor's sandbox. The first call creates it.
async function openSandbox(c: SandboxActorContext, state: BuildState): Promise<Sandbox> {
	const existing = state.sandboxId ? await provider.connect(c, state.sandboxId) : undefined;
	if (existing) return existing;
	state.sandboxId = await provider.create(c);
	const sandbox = await provider.connect(c, state.sandboxId);
	if (!sandbox) throw new Error(`Sandbox ${state.sandboxId} was not found right after it was created.`);
	return sandbox;
}

export const registry = setup({ use: { build } });

registry.start();

import { FileSystem } from "@opencode/core/filesystem";
import { FileSystemSearch } from "@opencode/core/filesystem/search";
import { PersistentPty } from "@opencode/core/persistent-pty/index";
import { ShellSelect } from "@opencode/core/shell/select";
import { Vcs } from "@opencode/core/vcs";
import type { LayerNode } from "@opencode/util/effect/layer-node";
import { Effect, Layer } from "effect";

/** Like the workerd profile, disable services which bypass Environment. */
export function sandboxProfile(): LayerNode.Replacements {
	const unavailable = () =>
		Effect.die(
			new Error(
				"This OpenCode host-only capability is unavailable with a remote sandbox",
			),
		);
	const terminalUnavailable = () =>
		Effect.fail(
			new PersistentPty.UnavailableError({
				message:
					"Persistent PTYs are unavailable through sandbox-adapter; use session.shell",
			}),
		);
	return [
		FileSystem.node.replace(
			Layer.succeed(FileSystem.Service, {
				read: unavailable,
				list: unavailable,
				find: unavailable,
			}),
		),
		FileSystemSearch.node.replace(
			Layer.succeed(FileSystemSearch.Service, { find: unavailable }),
		),
		Vcs.node.replace(
			Layer.succeed(Vcs.Service, {
				base: () => Effect.succeed(null),
				transform: () => Effect.succeed({ dispose: Effect.void }),
				reload: () => Effect.void,
				info: () => Effect.succeed({ branch: {} }),
				branches: () => Effect.succeed([]),
				status: () => Effect.succeed([]),
				diff: () => Effect.succeed([]),
			}),
		),
		ShellSelect.node.replace(
			Layer.succeed(ShellSelect.Service, {
				resolve: () => Effect.succeed("/bin/sh"),
				reload: () => Effect.void,
				transform: () => Effect.succeed({ dispose: Effect.void }),
			}),
		),
		PersistentPty.node.replace(
			Layer.succeed(PersistentPty.Service, {
				list: () => Effect.succeed([]),
				read: () => Effect.succeed(null),
				shutdown: () => Effect.void,
				handoff: () => Effect.succeed(null),
				get: terminalUnavailable,
				create: terminalUnavailable,
				write: terminalUnavailable,
				resize: terminalUnavailable,
				control: terminalUnavailable,
				input: terminalUnavailable,
				snapshot: terminalUnavailable,
				remove: terminalUnavailable,
				attach: terminalUnavailable,
			}),
		),
	];
}

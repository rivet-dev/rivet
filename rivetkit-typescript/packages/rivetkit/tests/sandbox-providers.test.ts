import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { SandboxProvider } from "sandbox-agent";
import { describe, expect, test } from "vitest";
import { sandboxActor } from "../src/sandbox/index";
import {
	SANDBOX_AGENT_ACTION_METHODS,
	SANDBOX_AGENT_HOOK_METHODS,
} from "../src/sandbox/types";

// --- SDK parity tests ---

function getPublicSandboxAgentSdkMethods(): string[] {
	let dir = path.dirname(fileURLToPath(import.meta.url));
	let declarationsPath: string | null = null;

	while (dir !== path.dirname(dir)) {
		const candidate = path.join(
			dir,
			"node_modules/sandbox-agent/dist/index.d.ts",
		);
		if (fs.existsSync(candidate)) {
			declarationsPath = candidate;
			break;
		}
		dir = path.dirname(dir);
	}

	if (!declarationsPath) {
		throw new Error("unable to locate sandbox-agent declarations");
	}

	const declarations = fs.readFileSync(declarationsPath, "utf8");
	const match = declarations.match(
		/declare class SandboxAgent \{([\s\S]*?)^\}/m,
	);
	if (!match) {
		throw new Error("unable to locate SandboxAgent declaration block");
	}

	return match[1]
		.split("\n")
		.map((line) => line.trim())
		.filter((line) => line.length > 0)
		.filter((line) => !line.startsWith("private "))
		.filter((line) => !line.startsWith("static "))
		.map((line) => line.match(/^([A-Za-z0-9_]+)\(/)?.[1] ?? null)
		.filter(
			(name): name is string => name !== null && name !== "constructor",
		)
		.sort();
}

describe("sandbox actor sdk parity", () => {
	test("keeps the hook and action split in sync with sandbox-agent", () => {
		expect(
			[
				...SANDBOX_AGENT_HOOK_METHODS,
				...SANDBOX_AGENT_ACTION_METHODS,
			].sort(),
		).toEqual(getPublicSandboxAgentSdkMethods());
	});

	test("exposes every sandbox-agent action method on the actor definition", () => {
		const providerStub: SandboxProvider = {
			name: "stub",
			async create() {
				throw new Error("not implemented");
			},
			async destroy() {
				throw new Error("not implemented");
			},
			async getUrl() {
				throw new Error("not implemented");
			},
		};
		const definition = sandboxActor({
			provider: providerStub,
		});

		const actionKeys = Object.keys(definition.config.actions ?? {}).sort();
		// The sandbox actor adds custom actions alongside all proxied
		// sandbox-agent methods.
		expect(actionKeys).toEqual(
			[
				...SANDBOX_AGENT_ACTION_METHODS,
				"destroy",
				"getSandboxUrl",
			].sort(),
		);
	});
});

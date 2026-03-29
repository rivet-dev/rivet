import { readFileSync, realpathSync } from "node:fs";
import { join, dirname } from "node:path";
import type { Kernel } from "@secure-exec/core";
import type { AgentConfig, PrepareInstructionsOptions } from "./agents.js";

// ── Software Descriptor Types ────────────────────────────────────────

export interface SoftwareDescriptor {
	name: string;
	type: "agent" | "tool" | "wasm-commands";
}

export interface AgentSoftwareDescriptor extends SoftwareDescriptor {
	type: "agent";
	/**
	 * Root directory of this npm package on the host. Used to resolve
	 * dependencies listed in `requires` from this package's node_modules/.
	 */
	packageDir: string;
	/** npm packages that must be available inside the VM. */
	requires: string[];
	agent: {
		/** Unique agent ID used in createSession(id). */
		id: string;
		/** npm package name of the ACP adapter. Must be in requires. */
		acpAdapter: string;
		/** npm package name of the agent CLI. Must be in requires. */
		agentPackage: string;
		/** Static env vars passed when spawning the adapter. */
		staticEnv?: Record<string, string>;
		/** Dynamic env vars computed at boot time. */
		env?: (ctx: SoftwareContext) => Record<string, string>;
		/**
		 * Prepare agent-specific spawn overrides for OS instruction injection.
		 * When provided, replaces the default instruction injection behavior.
		 */
		prepareInstructions?: AgentConfig["prepareInstructions"];
	};
}

export interface ToolSoftwareDescriptor extends SoftwareDescriptor {
	type: "tool";
	/**
	 * Root directory of this npm package on the host. Used to resolve
	 * dependencies listed in `requires` from this package's node_modules/.
	 */
	packageDir: string;
	/** npm packages that must be available inside the VM. */
	requires: string[];
	/** Map of bin command name -> npm package name. */
	bins: Record<string, string>;
}

export interface WasmCommandSoftwareDescriptor extends SoftwareDescriptor {
	type: "wasm-commands";
	/** Absolute path to directory containing WASM command binaries on the host. */
	commandDir: string;
	/** Symlink aliases: aliasName -> targetCommandName. */
	aliases?: Record<string, string>;
	/** Permission tier assignments. */
	permissions?: {
		full?: string[];
		readWrite?: string[];
		readOnly?: string[] | "*";
	};
}

/**
 * Any object with a commandDir property is treated as a WASM command package.
 * This allows registry packages (e.g., @rivet-dev/agent-os-coreutils) to be
 * passed directly to the `software` option without wrapping.
 */
export interface WasmCommandDirDescriptor {
	readonly commandDir: string;
	[key: string]: unknown;
}

export type AnySoftwareDescriptor =
	| AgentSoftwareDescriptor
	| ToolSoftwareDescriptor
	| WasmCommandSoftwareDescriptor
	| WasmCommandDirDescriptor;

/** Input type for the `software` option. Accepts descriptors or arrays of descriptors (for meta-packages). */
export type SoftwareInput = AnySoftwareDescriptor | AnySoftwareDescriptor[];

// ── SoftwareContext ───────────────────────────────────────────────────

export interface SoftwareContext {
	/**
	 * Resolve the bin entry for an npm package to a VM-side path.
	 * Uses require.resolve on the HOST, then maps to /root/node_modules/...
	 *
	 * Example: ctx.resolveBin("@mariozechner/pi-coding-agent", "pi")
	 *   -> "/root/node_modules/@mariozechner/pi-coding-agent/dist/cli.js"
	 */
	resolveBin(packageName: string, binName?: string): string;

	/**
	 * Resolve a package's root directory to a VM-side path.
	 *
	 * Example: ctx.resolvePackage("pi-acp")
	 *   -> "/root/node_modules/pi-acp"
	 */
	resolvePackage(packageName: string): string;
}

/** Host-to-VM path mapping for ModuleAccessFileSystem. */
export interface SoftwareRoot {
	hostPath: string;
	vmPath: string;
}

/**
 * Create a SoftwareContext for a software descriptor.
 * Resolves npm package paths relative to the descriptor's packageDir.
 */
function createSoftwareContext(
	packageDir: string,
	requires: string[],
): SoftwareContext {
	// Pre-resolve all required packages to host paths
	const resolvedPackages = new Map<
		string,
		{ hostDir: string; vmDir: string; pkg: Record<string, unknown> }
	>();

	for (const reqPkg of requires) {
		const hostPkgJsonPath = join(
			packageDir,
			"node_modules",
			reqPkg,
			"package.json",
		);
		try {
			const pkg = JSON.parse(readFileSync(hostPkgJsonPath, "utf-8"));
			const hostDir = dirname(hostPkgJsonPath);
			const vmDir = `/root/node_modules/${reqPkg}`;
			resolvedPackages.set(reqPkg, { hostDir, vmDir, pkg });
		} catch {
			throw new Error(
				`Package "${reqPkg}" not found at ${hostPkgJsonPath}. ` +
					`Ensure it is listed as a dependency of the package at ${packageDir}.`,
			);
		}
	}

	return {
		resolveBin(packageName: string, binName?: string): string {
			const resolved = resolvedPackages.get(packageName);
			if (!resolved) {
				throw new Error(
					`Package "${packageName}" is not in the requires list. ` +
						`Available: ${[...resolvedPackages.keys()].join(", ")}`,
				);
			}

			const { pkg, vmDir } = resolved;
			let binEntry: string | undefined;
			const effectiveBinName = binName ?? packageName;

			if (typeof pkg.bin === "string") {
				binEntry = pkg.bin;
			} else if (typeof pkg.bin === "object" && pkg.bin !== null) {
				const binMap = pkg.bin as Record<string, string>;
				binEntry = binMap[effectiveBinName] ?? Object.values(binMap)[0];
			}

			if (!binEntry) {
				throw new Error(
					`No bin entry "${effectiveBinName}" found in ${packageName}/package.json`,
				);
			}

			return `${vmDir}/${binEntry}`;
		},

		resolvePackage(packageName: string): string {
			const resolved = resolvedPackages.get(packageName);
			if (!resolved) {
				throw new Error(
					`Package "${packageName}" is not in the requires list. ` +
						`Available: ${[...resolvedPackages.keys()].join(", ")}`,
				);
			}
			return resolved.vmDir;
		},
	};
}

// ── defineSoftware ───────────────────────────────────────────────────

/**
 * Define a software descriptor. This is a type-safe identity function that
 * validates the descriptor shape at compile time.
 */
export function defineSoftware<T extends AnySoftwareDescriptor>(desc: T): T {
	return desc;
}

// ── Software Processing ──────────────────────────────────────────────

/** Result of processing all software descriptors at boot time. */
export interface ProcessedSoftware {
	/** WASM command directories to pass to the WasmVM driver. */
	commandDirs: string[];
	/** Host-to-VM path mappings for ModuleAccessFileSystem. */
	softwareRoots: SoftwareRoot[];
	/** Agent configs registered by agent software. */
	agentConfigs: Map<string, AgentConfig>;
}

/** Check if a descriptor is a typed software descriptor (has a `type` field). */
function isTypedDescriptor(desc: AnySoftwareDescriptor): desc is AgentSoftwareDescriptor | ToolSoftwareDescriptor | WasmCommandSoftwareDescriptor {
	return "type" in desc && typeof (desc as SoftwareDescriptor).type === "string";
}

/**
 * Process an array of software descriptors at boot time.
 * Collects WASM command dirs, module access roots, and agent configurations.
 *
 * Any object with a `commandDir` property (e.g., registry packages) is treated
 * as a WASM command source. Typed descriptors with `type: "agent"` or `type: "tool"`
 * are processed for module mounting and agent registration.
 */
export function processSoftware(
	software: SoftwareInput[],
): ProcessedSoftware {
	const commandDirs: string[] = [];
	const softwareRoots: SoftwareRoot[] = [];
	const agentConfigs = new Map<string, AgentConfig>();

	// Flatten nested arrays (meta-packages export arrays of sub-packages).
	const flat = software.flat() as AnySoftwareDescriptor[];

	for (const pkg of flat) {
		if (!isTypedDescriptor(pkg)) {
			// Duck-typed: any object with commandDir is a WASM command source.
			commandDirs.push(pkg.commandDir);
			continue;
		}

		switch (pkg.type) {
			case "wasm-commands": {
				commandDirs.push(pkg.commandDir);
				break;
			}

			case "agent": {
				// Collect module roots for all required npm packages.
				// Resolve symlinks so pnpm's linked node_modules work correctly.
				const ctx = createSoftwareContext(pkg.packageDir, pkg.requires);
				for (const reqPkg of pkg.requires) {
					const hostDir = realpathSync(join(
						pkg.packageDir,
						"node_modules",
						reqPkg,
					));
					const vmDir = `/root/node_modules/${reqPkg}`;
					softwareRoots.push({ hostPath: hostDir, vmPath: vmDir });
				}

				// Compute static + dynamic env vars.
				const staticEnv = pkg.agent.staticEnv ?? {};
				const dynamicEnv = pkg.agent.env ? pkg.agent.env(ctx) : {};
				const combinedEnv = { ...staticEnv, ...dynamicEnv };

				// Register agent config.
				const agentConfig: AgentConfig = {
					acpAdapter: pkg.agent.acpAdapter,
					agentPackage: pkg.agent.agentPackage,
					defaultEnv: Object.keys(combinedEnv).length > 0 ? combinedEnv : undefined,
					prepareInstructions: pkg.agent.prepareInstructions,
				};

				agentConfigs.set(pkg.agent.id, agentConfig);
				break;
			}

			case "tool": {
				// Collect module roots for all required npm packages.
				// Resolve symlinks so pnpm's linked node_modules work correctly.
				for (const reqPkg of pkg.requires) {
					const hostDir = realpathSync(join(
						pkg.packageDir,
						"node_modules",
						reqPkg,
					));
					const vmDir = `/root/node_modules/${reqPkg}`;
					softwareRoots.push({ hostPath: hostDir, vmPath: vmDir });
				}
				// Tool bin registration is handled by the caller (AgentOs.create)
				// since it requires kernel access.
				break;
			}
		}
	}

	return { commandDirs, softwareRoots, agentConfigs };
}

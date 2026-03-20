import { createRequire } from "node:module";
import path from "node:path";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface CompileActorSourceOptions {
	/** TypeScript source text. */
	source: string;

	/** Filename hint for diagnostics (default: "actor.ts"). */
	filename?: string;

	/** Output module format (default: "esm"). */
	format?: "esm" | "commonjs";

	/** Run the full type checker (default: false). Strip-only when false. */
	typecheck?: boolean;

	/** Additional tsconfig compilerOptions overrides. */
	compilerOptions?: Record<string, unknown>;

	/** Memory limit for the compiler isolate in MB (default: 512). */
	memoryLimit?: number;

	/** CPU time limit for the compiler isolate in ms. */
	cpuTimeLimitMs?: number;
}

export interface CompileActorSourceResult {
	/** Compiled JavaScript output. Undefined if compilation failed. */
	js?: string;

	/** Source map text, if generated. */
	sourceMap?: string;

	/** Whether compilation succeeded without errors. */
	success: boolean;

	/** TypeScript diagnostics (errors, warnings, suggestions). */
	diagnostics: TypeScriptDiagnostic[];
}

export interface TypeScriptDiagnostic {
	code: number;
	category: "error" | "warning" | "suggestion" | "message";
	message: string;
	line?: number;
	column?: number;
}

// ---------------------------------------------------------------------------
// Internal types mirroring the @secure-exec/typescript and secure-exec APIs.
// Kept local so the packages remain dynamically loaded at runtime only.
// ---------------------------------------------------------------------------

interface SecureExecTypescriptModule {
	createTypeScriptTools: (options: {
		systemDriver: unknown;
		runtimeDriverFactory: unknown;
		memoryLimit?: number;
		cpuTimeLimitMs?: number;
	}) => {
		compileSource: (options: {
			sourceText: string;
			filePath?: string;
			compilerOptions?: Record<string, unknown>;
		}) => Promise<{
			success: boolean;
			outputText?: string;
			sourceMapText?: string;
			diagnostics: TypeScriptDiagnostic[];
		}>;
	};
}

interface SecureExecCoreModule {
	createNodeDriver: (options: Record<string, unknown>) => unknown;
	createNodeRuntimeDriverFactory: () => unknown;
}

// ---------------------------------------------------------------------------
// Module loading — uses the build-specifier-from-parts pattern to prevent
// bundlers from eagerly including these optional packages.
// ---------------------------------------------------------------------------

let secureExecTsModulePromise: Promise<SecureExecTypescriptModule> | undefined;
let secureExecCoreModulePromise: Promise<SecureExecCoreModule> | undefined;

function createRuntimeRequire(): NodeRequire {
	return createRequire(import.meta.url);
}

function resolveEsmPackageEntry(packageName: string): string | undefined {
	let current = process.cwd();
	while (true) {
		const pkgJsonPath = path.join(
			current,
			"node_modules",
			packageName,
			"package.json",
		);
		try {
			const content = readFileSync(pkgJsonPath, "utf-8");
			const pkgJson = JSON.parse(content) as {
				main?: string;
				exports?: Record<string, unknown>;
			};
			const entryRelative =
				(pkgJson.exports?.["."] as { import?: string } | undefined)
					?.import ?? pkgJson.main;
			if (entryRelative) {
				return path.resolve(path.dirname(pkgJsonPath), entryRelative);
			}
		} catch {
			// package.json not found at this level, keep walking up
		}
		const parent = path.dirname(current);
		if (parent === current) break;
		current = parent;
	}
	return undefined;
}

function resolvePackageEntry(packageName: string): string {
	const resolver = createRuntimeRequire();
	try {
		return resolver.resolve(packageName);
	} catch {
		const resolved = resolveEsmPackageEntry(packageName);
		if (resolved) return resolved;
		throw new Error(
			`Cannot resolve package "${packageName}". Install it as a dependency.`,
		);
	}
}

async function nativeDynamicImport<T>(specifier: string): Promise<T> {
	try {
		return (await import(specifier)) as T;
	} catch (directError) {
		const importer = new Function(
			"moduleSpecifier",
			"return import(moduleSpecifier);",
		) as (moduleSpecifier: string) => Promise<T>;
		try {
			return await importer(specifier);
		} catch {
			throw directError;
		}
	}
}

async function loadSecureExecTypescriptModule(): Promise<SecureExecTypescriptModule> {
	if (!secureExecTsModulePromise) {
		secureExecTsModulePromise = (async () => {
			// Build specifier from parts to avoid bundler eager inclusion.
			const specifier = ["@secure-exec", "typescript"].join("/");
			const entryPath = resolvePackageEntry(specifier);
			const entrySpecifier = pathToFileURL(entryPath).href;
			return await nativeDynamicImport<SecureExecTypescriptModule>(
				entrySpecifier,
			);
		})();
	}
	return secureExecTsModulePromise;
}

async function loadSecureExecCoreModule(): Promise<SecureExecCoreModule> {
	if (!secureExecCoreModulePromise) {
		secureExecCoreModulePromise = (async () => {
			// Build specifier from parts to avoid bundler eager inclusion.
			const specifier = ["secure", "exec"].join("-");
			const entryPath = resolvePackageEntry(specifier);
			const entrySpecifier = pathToFileURL(entryPath).href;
			return await nativeDynamicImport<SecureExecCoreModule>(
				entrySpecifier,
			);
		})();
	}
	return secureExecCoreModulePromise;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export async function compileActorSource(
	options: CompileActorSourceOptions,
): Promise<CompileActorSourceResult> {
	const [secureExecTs, secureExecCore] = await Promise.all([
		loadSecureExecTypescriptModule(),
		loadSecureExecCoreModule(),
	]);

	const systemDriver = secureExecCore.createNodeDriver({});
	const runtimeDriverFactory =
		secureExecCore.createNodeRuntimeDriverFactory();

	const tools = secureExecTs.createTypeScriptTools({
		systemDriver,
		runtimeDriverFactory,
		memoryLimit: options.memoryLimit,
		cpuTimeLimitMs: options.cpuTimeLimitMs,
	});

	const compilerOptions: Record<string, unknown> = {
		...options.compilerOptions,
	};

	// Set module format.
	if (options.format === "commonjs") {
		compilerOptions.module ??= "commonjs";
	} else {
		// Default to ESM.
		compilerOptions.module ??= "esnext";
		compilerOptions.moduleResolution ??= "bundler";
	}

	// When typecheck is false, use noCheck to strip types without running the
	// checker. This is substantially faster.
	if (!options.typecheck) {
		compilerOptions.noCheck = true;
	}

	const result = await tools.compileSource({
		sourceText: options.source,
		filePath: options.filename ?? "actor.ts",
		compilerOptions,
	});

	return {
		js: result.outputText,
		sourceMap: result.sourceMapText,
		success: result.success,
		diagnostics: result.diagnostics.map((d) => ({
			code: d.code,
			category: d.category,
			message: d.message,
			line: d.line,
			column: d.column,
		})),
	};
}

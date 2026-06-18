import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "vitest";

// Regression guard for rivetkit issue #2:
//   "import.meta.url in CJS chunks crashes ts-node/CJS loaders"
//
// Background: tsup emits CommonJS (.cjs) bundles for the `require` conditions
// of every package export. `import.meta` is an ESM-only syntactic form; when a
// raw `import.meta.url` leaks into a .cjs file, loading that file under a
// CommonJS loader (ts-node, plain `require()`, older bundlers) throws a
// SyntaxError ("Cannot use 'import.meta' outside a module"). The fix is
// `shims: true` in the shared tsup config (tsup.base.ts), which rewrites
// `import.meta.url` into a CJS-safe shim (e.g. `new URL(\`file:${__filename}\`).href`).
//
// This test statically scans every produced .cjs file in the built dist/tsup
// output and asserts that none of them contains the literal `import.meta.url`.
// It directly encodes the original failure mode: if `shims` regresses (or a new
// reachable usage slips past tree-shaking), the offending bundle will contain
// the raw token and this test will fail before it ever crashes a CJS consumer.
//
// Approach chosen: static build-output assertion (the lightest reliable check).
// It requires the package to be built. If dist/tsup has not been built yet we
// throw with an actionable message rather than silently passing, so a missing
// build is never mistaken for "no occurrences found".

const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const PACKAGE_DIR = resolve(TEST_DIR, "..");
const DIST_TSUP_DIR = resolve(PACKAGE_DIR, "dist", "tsup");

const FORBIDDEN_TOKEN = "import.meta.url";

function isDirectory(path: string): boolean {
	try {
		return statSync(path).isDirectory();
	} catch {
		return false;
	}
}

function collectCjsFiles(root: string): string[] {
	const results: string[] = [];
	const stack: string[] = [root];
	while (stack.length > 0) {
		const current = stack.pop();
		if (current === undefined) continue;
		for (const entry of readdirSync(current, { withFileTypes: true })) {
			const fullPath = resolve(current, entry.name);
			if (entry.isDirectory()) {
				stack.push(fullPath);
			} else if (entry.isFile() && entry.name.endsWith(".cjs")) {
				results.push(fullPath);
			}
		}
	}
	return results;
}

describe("rivetkit CJS bundles are free of raw import.meta (issue #2)", () => {
	test("no built .cjs file contains a raw import.meta.url token", () => {
		if (!isDirectory(DIST_TSUP_DIR)) {
			throw new Error(
				`Expected built CJS output at ${DIST_TSUP_DIR} but it does not exist. ` +
					`Run \`pnpm run build\` in ${PACKAGE_DIR} before running this regression guard.`,
			);
		}

		const cjsFiles = collectCjsFiles(DIST_TSUP_DIR);

		// Sanity: the build must have produced at least one CJS bundle. If it
		// produced none, the scan below would be vacuously green, which would
		// silently mask a regression.
		expect(
			cjsFiles.length,
			`Expected at least one .cjs bundle under ${DIST_TSUP_DIR}; found none. ` +
				`The build may be incomplete.`,
		).toBeGreaterThan(0);

		const offenders: string[] = [];
		for (const file of cjsFiles) {
			const contents = readFileSync(file, "utf8");
			if (contents.includes(FORBIDDEN_TOKEN)) {
				offenders.push(file.slice(PACKAGE_DIR.length + 1));
			}
		}

		expect(
			offenders,
			`Found raw \`${FORBIDDEN_TOKEN}\` in ${offenders.length} built CJS file(s). ` +
				`import.meta is ESM-only and crashes CommonJS/ts-node loaders. ` +
				`Ensure \`shims: true\` remains set in tsup.base.ts. Offending files:\n` +
				offenders.map((f) => `  - ${f}`).join("\n"),
		).toEqual([]);
	});
});

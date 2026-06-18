import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { minimatch } from "minimatch";
import { describe, expect, test } from "vitest";

/**
 * Regression guard for rivetkit issue #3: "Missing CJS wrappers for declared exports".
 *
 * Root cause that remains: @rivetkit/engine-runner-protocol declares CJS export
 * targets (require.default -> ./dist/index.cjs, require.types -> ./dist/index.d.cts)
 * but its package.json `files` field is the restrictive glob
 * ["dist/**\/*.js", "dist/**\/*.d.ts"], which excludes .cjs and .d.cts. As a result
 * the published tarball does NOT contain the declared require target, so a CJS
 * consumer doing `require("@rivetkit/engine-runner-protocol")` crashes with
 * ERR_MODULE_NOT_FOUND / cannot find module ./dist/index.cjs.
 *
 * This is a purely STATIC check (no build / no network): it reads package.json,
 * enumerates every leaf target path in `exports`, and asserts each one is included
 * by the package's `files` field using npm's packing glob semantics. It encodes
 * the CORRECT expected behavior, so it FAILS while the bug is present and will
 * PASS once `files` is widened to ship the declared CJS artifacts.
 */

const here = dirname(fileURLToPath(import.meta.url));
const pkgDir = resolve(here, "..");
const pkgJsonPath = join(pkgDir, "package.json");

interface PkgJson {
	name: string;
	exports?: unknown;
	files?: string[];
}

const pkg: PkgJson = JSON.parse(readFileSync(pkgJsonPath, "utf8"));

/** Recursively collect every leaf string value from an exports map. */
function collectExportTargets(node: unknown, acc: Set<string>): void {
	if (typeof node === "string") {
		// Only consider relative file targets (skip e.g. bare specifiers).
		if (node.startsWith("./") || node.startsWith("dist/")) {
			acc.add(node.replace(/^\.\//, ""));
		}
		return;
	}
	if (Array.isArray(node)) {
		for (const child of node) collectExportTargets(child, acc);
		return;
	}
	if (node && typeof node === "object") {
		for (const value of Object.values(node as Record<string, unknown>)) {
			collectExportTargets(value, acc);
		}
	}
}

/**
 * Does the npm `files` field include `targetPath`?
 *
 * npm semantics: a bare path that names a directory (e.g. "dist") behaves like
 * "dist/**" (the whole subtree is included). Otherwise the entry is treated as a
 * glob and matched against the package-relative path.
 */
function filesIncludes(files: string[], targetPath: string): boolean {
	for (const raw of files) {
		const pattern = raw.replace(/^\.\//, "").replace(/\/$/, "");
		if (minimatch(targetPath, pattern)) return true;
		// Bare directory name -> recursive include.
		if (!pattern.includes("*")) {
			if (
				targetPath === pattern ||
				targetPath.startsWith(`${pattern}/`)
			) {
				return true;
			}
		}
	}
	return false;
}

describe("engine-runner-protocol exports are shippable via files field", () => {
	const targets = new Set<string>();
	collectExportTargets(pkg.exports, targets);
	const files = pkg.files ?? [];

	test("package declares exports and a files field", () => {
		expect(targets.size).toBeGreaterThan(0);
		expect(files.length).toBeGreaterThan(0);
	});

	for (const target of targets) {
		test(`exports target "${target}" is included by the files field (so it is published)`, () => {
			expect(
				filesIncludes(files, target),
				`package.json "files" (${JSON.stringify(files)}) does not include exports target "${target}". ` +
					`A CJS/ESM consumer that resolves to "${target}" would crash with ERR_MODULE_NOT_FOUND ` +
					`because the file is excluded from the published tarball (issue #3).`,
			).toBe(true);
		});
	}

	test("declared exports targets exist on disk after build (when dist is present)", () => {
		// Soft check: only assert disk presence if the package has been built.
		const anyBuilt = [...targets].some((t) => existsSync(join(pkgDir, t)));
		if (!anyBuilt) {
			// Not built locally; the files-vs-exports static checks above are the
			// authoritative guard for issue #3.
			return;
		}
		for (const target of targets) {
			expect(
				existsSync(join(pkgDir, target)),
				`exports target "${target}" is missing from dist after build`,
			).toBe(true);
		}
	});
});

import type { ReleaseOpts } from "./main.ts";
import { assertStringIncludes } from "@std/assert";
import $ from "dax";

async function npmVersionExists(
	packageName: string,
	version: string,
): Promise<boolean> {
	$.logStep("Checking if NPM version exists", `${packageName}@${version}`);
	const npmCheck = await $`npm view ${packageName}@${version} version`.quiet().noThrow();
	if (npmCheck.code === 0) {
		return true;
	} else {
		console.log('out', npmCheck.stdout);
		assertStringIncludes(npmCheck.stderr, `No match found for version ${version}`, "unexpected output");
		return false;
	}
}

async function jsrVersionExists(
	packageName: string,
	version: string,
): Promise<boolean> {
	$.logStep("Checking if JSR version exists", `${packageName}@${version}`);
	const denoCheck = await $`deno info jsr:${packageName}@${version}`.quiet().noThrow();
	if (denoCheck.code === 0) {
		return true;
	} else {
		assertStringIncludes(denoCheck.stderr, `Could not find version of '${packageName}' that matches specified version constraint '${version}'`, "unexpected output");
		return false;
	}
}

export async function publishSdk(opts: ReleaseOpts) {
	const packages = [
		{ path: `${opts.root}/sdks/api/runtime/typescript`, name: "@rivet-gg/api", npm: true },
		{ path: `${opts.root}/sdks/api/full/typescript`, name: "@rivet-gg/api-full", npm: true },
		{ path: `${opts.root}/sdks/actor/runtime`, name: "@rivet-gg/actor", jsr: true },
		{ path: `${opts.root}/sdks/actor/client`, name: "@rivet-gg/actor-client", jsr: true },
		{ path: `${opts.root}/sdks/actor/core`, name: "@rivet-gg/actor-core", jsr: true },
	];

	for (const pkg of packages) {
		// Check if version already exists
		let versionExists = false;
		if (pkg.npm) {
			versionExists = await npmVersionExists(pkg.name, opts.version);
		} else if (pkg.jsr) {
			versionExists = await jsrVersionExists(pkg.name, opts.version);
		}

		if (versionExists) {
			$.logLight(
				`Version ${opts.version} of ${pkg.name} already exists. Skipping...`,
			);
			continue;
		}

		// Publish
		if (pkg.npm) {
			$.logStep("Publishing to NPM", `${pkg.name}@${opts.version}`);
			await $`npm version ${opts.version} --no-git-tag-version --allow-same-version`.cwd(pkg.path);
			await $`npm publish`.cwd(pkg.path)
		}

		if (pkg.jsr) {
			$.logStep("Publishing to JSR", `${pkg.name}@${opts.version}`);

			// TODO(https://github.com/denoland/deno/issues/27428): `--set-version` not working, so we have to manually update `jsr.jsonc`
			// Update version in config
			const jsrJsonPath = `${pkg.path}/deno.json`;
			const jsrJsonContent = await Deno.readTextFile(jsrJsonPath);
			const jsrJson = JSON.parse(jsrJsonContent);
			jsrJson.version = opts.version;
			await Deno.writeTextFile(jsrJsonPath, JSON.stringify(jsrJson, null, 2));

			// TODO: Auto-populate token here
			// --allow-slow-types = we use zod which doesn't support this
			// --allow-dirty = we change the version on the fly
			// --set-version = validate the correct version is used
			await $`deno publish --allow-slow-types --allow-dirty --set-version ${opts.version}`.cwd(pkg.path);
		}
	}
}

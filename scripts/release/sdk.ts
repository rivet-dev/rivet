import type { ReleaseOpts } from "./main.ts";
import { assertStringIncludes } from "@std/assert";
import $ from "dax";
import { transformPackageJsonToDenoJson } from "../sdk_actor/transform_pkg_to_deno.ts";

async function npmVersionExists(
	packageName: string,
	version: string,
): Promise<boolean> {
	$.logStep("Checking if NPM version exists", `${packageName}@${version}`);
	const npmCheck = await $`npm view ${packageName}@${version} version`.quiet()
		.noThrow();
	if (npmCheck.code === 0) {
		return true;
	} else {
		assertStringIncludes(
			npmCheck.stderr,
			`No match found for version ${version}`,
			"unexpected output",
		);
		return false;
	}
}

async function jsrVersionExists(
	packageName: string,
	version: string,
): Promise<boolean> {
	$.logStep("Checking if JSR version exists", `${packageName}@${version}`);
	const denoCheck = await $`deno info jsr:${packageName}@${version}`.quiet()
		.noThrow();
	if (denoCheck.code === 0) {
		return true;
	} else {
		assertStringIncludes(
			denoCheck.stderr,
			`Could not find version of '${packageName}' that matches specified version constraint '${version}'`,
			"unexpected output",
		);
		return false;
	}
}

export async function publishSdk(opts: ReleaseOpts) {
	const packages: { path: string, name: string, npm: boolean, jsr?: boolean, turbo?: boolean }[] = [
		{
			path: `${opts.root}/sdks/typescript/api-runtime`,
			name: "@rivet-gg/api",
			npm: true,
		},
		{
			path: `${opts.root}/sdks/typescript/api-full`,
			name: "@rivet-gg/api-full",
			npm: true,
		},
	];

	for (const pkg of packages) {
		if (pkg.turbo) {
			await $`pnpm build --filter ${pkg.name}`;
		}

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
			await $`pnpm install`.cwd(pkg.path);
			await $`pnpm npm publish --access public --tolerate-republish`.cwd(pkg.path);
		}

		if (pkg.jsr) {
			$.logStep("Publishing to JSR", `${pkg.name}@${opts.version}`);

			// TODO(https://github.com/denoland/deno/issues/27428): `--set-version` not working, so we have to manually update `jsr.jsonc`

			await transformPackageJsonToDenoJson({
				cwd: pkg.path,
				skipPathInInternalPackages: "src",
				internalPackagesLinkPath: "internal",
			});

			// TODO: Auto-populate token here
			// --allow-slow-types = we use zod which doesn't support this
			// --allow-dirty = we change the version on the fly
			// --set-version = validate the correct version is used
			await $`DENO_NO_PACKAGE_JSON=1 deno publish --allow-slow-types --allow-dirty`
				.cwd(pkg.path);
		}
	}
}

import type { ReleaseOpts } from "./main.ts";
import { $ } from "execa";
import { transformPackageJsonToDenoJson } from "./transform_pkg_to_deno.ts";

function assertStringIncludes(actual: string, expected: string, message?: string): void {
	if (!actual.includes(expected)) {
		throw new Error(message || `String does not include expected substring: ${expected}`);
	}
}

async function npmVersionExists(
	packageName: string,
	version: string,
): Promise<boolean> {
	console.log(`==> Checking if NPM version exists: ${packageName}@${version}`);
	try {
		await $({ stdout: 'ignore', stderr: 'pipe' })`npm view ${packageName}@${version} version`;
		return true;
	} catch (error: any) {
		if (error.stderr) {
			assertStringIncludes(
				error.stderr,
				`No match found for version ${version}`,
				"unexpected output",
			);
		}
		return false;
	}
}

async function jsrVersionExists(
	packageName: string,
	version: string,
): Promise<boolean> {
	console.log(`==> Checking if JSR version exists: ${packageName}@${version}`);
	try {
		await $({ stdout: 'ignore', stderr: 'pipe' })`deno info jsr:${packageName}@${version}`;
		return true;
	} catch (error: any) {
		if (error.stderr) {
			assertStringIncludes(
				error.stderr,
				`Could not find version of '${packageName}' that matches specified version constraint '${version}'`,
				"unexpected output",
			);
		}
		return false;
	}
}

export async function publishSdk(opts: ReleaseOpts) {
	const packages: { path: string, name: string, npm: boolean, jsr?: boolean, turbo?: boolean }[] = [
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
			console.log(
				`Version ${opts.version} of ${pkg.name} already exists. Skipping...`,
			);
			continue;
		}

		// Publish
		if (pkg.npm) {
			console.log(`==> Publishing to NPM: ${pkg.name}@${opts.version}`);
			await $({ cwd: pkg.path })`pnpm install`;
			await $({ cwd: pkg.path })`pnpm npm publish --access public --tolerate-republish`;
		}

		if (pkg.jsr) {
			console.log(`==> Publishing to JSR: ${pkg.name}@${opts.version}`);

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
			await $({ cwd: pkg.path, env: { DENO_NO_PACKAGE_JSON: '1' } })`deno publish --allow-slow-types --allow-dirty`;
		}
	}
}

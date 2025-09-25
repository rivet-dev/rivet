import type { ReleaseOpts } from "./main";
import { $ } from "execa";
import { readFile } from "fs/promises";
import { join } from "path";

async function npmVersionExists(
	packageName: string,
	version: string,
): Promise<boolean> {
	console.log(
		`==> Checking if NPM version exists: ${packageName}@${version}`,
	);
	try {
		await $({
			stdout: "ignore",
			stderr: "pipe",
		})`npm view ${packageName}@${version} version`;
		return true;
	} catch (error: any) {
		if (error.stderr) {
			if (
				!error.stderr.includes(
					`No match found for version ${version}`,
				) &&
				!error.stderr.includes(
					`'${packageName}@${version}' is not in this registry.`,
				)
			) {
				throw new Error(
					`unexpected npm view version output: ${error.stderr}`,
				);
			}
		}
		return false;
	}
}

export async function publishSdk(opts: ReleaseOpts) {
	const packagePaths = [
		`${opts.root}/sdks/typescript/runner`,
		`${opts.root}/sdks/typescript/runner-protocol`,
		`${opts.root}/sdks/typescript/api-full`,
	];

	for (const path of packagePaths) {
		// Read package.json to get the name
		const packageJsonPath = join(path, "package.json");
		const packageJson = JSON.parse(await readFile(packageJsonPath, "utf-8"));
		const name = packageJson.name;

		// Check if version already exists
		let versionExists = false;
		versionExists = await npmVersionExists(name, opts.version);

		if (versionExists) {
			console.log(
				`Version ${opts.version} of ${name} already exists. Skipping...`,
			);
			continue;
		}

		// Publish
		console.log(`==> Publishing to NPM: ${name}@${opts.version}`);

		// Add --tag flag for release candidates
		const isReleaseCandidate = opts.version.includes("-rc.");
		const tag = isReleaseCandidate ? "rc" : "latest";

		await $({
			stdio: "inherit",
		})`pnpm --filter ${name} publish --access public --tag ${tag}`;
	}
}

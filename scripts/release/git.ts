import { $ } from "execa";
import type { ReleaseOpts } from "./main";

export async function validateGit(_opts: ReleaseOpts) {
	// Validate there's no uncommitted changes
	const result = await $`git status --porcelain`;
	const status = result.stdout;
	if (status.trim().length > 0) {
		throw new Error(
			"There are uncommitted changes. Please commit or stash them.",
		);
	}
}

export async function createAndPushTag(opts: ReleaseOpts) {
	console.log(`Creating tag v${opts.version}...`);
	try {
		// Create tag and force update if it exists
		await $({ cwd: opts.root })`git tag -f v${opts.version}`;

		// Push tag with force to ensure it's updated
		await $({ cwd: opts.root })`git push origin v${opts.version} -f`;

		console.log(`✅ Tag v${opts.version} created and pushed`);
	} catch (err) {
		console.error("❌ Failed to create or push tag");
		throw err;
	}
}

export async function createGitHubRelease(opts: ReleaseOpts) {
	console.log("Creating GitHub release...");

	try {
		// Get the current tag name (should be the tag created during the release process)
		const { stdout: currentTag } = await $({
			cwd: opts.root,
		})`git describe --tags --exact-match`;
		const tagName = currentTag.trim();

		console.log(`Looking for existing release for ${opts.version}`);

		// Check if a release with this version name already exists
		const { stdout: releaseJson } = await $({
			cwd: opts.root,
		})`gh release list --json name,tagName`;
		const releases = JSON.parse(releaseJson);
		const existingRelease = releases.find(
			(r: any) => r.name === opts.version,
		);

		if (existingRelease) {
			console.log(
				`Updating release ${opts.version} to point to new tag ${tagName}`,
			);
			await $({
				cwd: opts.root,
			})`gh release edit ${existingRelease.tagName} --tag ${tagName}`;
		} else {
			console.log(
				`Creating new release ${opts.version} pointing to tag ${tagName}`,
			);
			await $({
				cwd: opts.root,
			})`gh release create ${tagName} --title ${opts.version} --generate-notes`;

			// Check if this is a pre-release (contains -rc. or similar)
			if (opts.version.includes("-")) {
				await $({
					cwd: opts.root,
				})`gh release edit ${tagName} --prerelease`;
			}
		}

		console.log("✅ GitHub release created/updated");
	} catch (err) {
		console.error("❌ Failed to create GitHub release");
		console.warn("! You may need to create the release manually");
		throw err;
	}
}

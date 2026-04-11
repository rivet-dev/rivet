/**
 * Git helpers for the release tail: clean-tree validation, tag + push, and
 * GitHub release creation.
 */
import { $ } from "execa";
import { scoped } from "./logger.js";

const log = scoped("git");

/** Refuse to proceed if there are uncommitted changes. */
export async function validateClean(): Promise<void> {
	const { stdout } = await $`git status --porcelain`;
	if (stdout.trim().length > 0) {
		throw new Error(
			"there are uncommitted changes — commit or stash them before proceeding",
		);
	}
}

/** Force-create and force-push `v{version}`. */
export async function tagAndPush(version: string): Promise<void> {
	log.info(`creating tag v${version}`);
	await $({ stdio: "inherit" })`git tag -f v${version}`;
	await $({ stdio: "inherit" })`git push origin v${version} -f`;
	log.info(`pushed v${version}`);
}

/**
 * Create (or update) a GitHub release for the version. If a release with the
 * same name already exists, its tag is updated to match. New releases with a
 * prerelease identifier (anything containing `-`) are marked as prerelease.
 */
export async function createGhRelease(version: string): Promise<void> {
	log.info(`creating GitHub release for ${version}`);

	const { stdout: currentTag } = await $`git describe --tags --exact-match`;
	const tagName = currentTag.trim();

	const { stdout: releaseJson } =
		await $`gh release list --json name,tagName --limit 200`;
	const releases = JSON.parse(releaseJson) as Array<{
		name: string;
		tagName: string;
	}>;
	const existing = releases.find((r) => r.name === version);

	if (existing) {
		log.info(`updating existing release ${version} -> tag ${tagName}`);
		await $({
			stdio: "inherit",
		})`gh release edit ${existing.tagName} --tag ${tagName}`;
		return;
	}

	log.info(`creating new release ${version} -> tag ${tagName}`);
	await $({
		stdio: "inherit",
	})`gh release create ${tagName} --title ${version} --generate-notes`;

	// Prerelease detection: anything with a `-` is a prerelease per semver.
	if (version.includes("-")) {
		await $({
			stdio: "inherit",
		})`gh release edit ${tagName} --prerelease`;
	}
}

import { assertEquals } from "@std/assert/equals";
import type { ReleaseOpts } from "./main.ts";
import $ from "dax";

export async function getCommit() {
	// Check if the current branch is 'main'
	const branch = await $`git rev-parse --abbrev-ref HEAD`.text();
	assertEquals(branch, "main", "must be on main branch to release");

	// Check if the local branch is up-to-date with the origin
	const localCommit = await $`git rev-parse HEAD`.text();
	const remoteCommit = await $`git rev-parse origin/main`.text();
	assertEquals(
		localCommit,
		remoteCommit,
		"your branch is not up to date with origin",
	);

	// Save short commit
	return localCommit.slice(0, 7);
}

export async function validateGit(opts: ReleaseOpts) {
	// Validate there's no uncommitted changes
	const status = await $`git status --porcelain`.text();
	if (status) {
		throw new Error(
			"There are uncommitted changes. Please commit or stash them.",
		);
	}
}

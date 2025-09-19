import type { ReleaseOpts } from "./main";
import { $ } from "execa";

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

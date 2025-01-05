import type { ReleaseOpts } from "./main.ts";
import $ from "dax";

export async function configureReleasePlease(opts: ReleaseOpts) {
	// Check if the Release-As commit already exists
	const commitMessage = `chore: release ${opts.version}`;
	$.logStep("Updating Release Please", commitMessage);
	await $`git commit --allow-empty -m ${commitMessage} -m ${`Release-As: ${opts.version}`}`;
}

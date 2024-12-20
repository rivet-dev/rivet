import type { ReleaseOpts } from "./main.ts";
import $ from "dax";

export async function configureReleasePlease(opts: ReleaseOpts) {
	// Check if the Release-As commit already exists
	const commitMessage = `chore: release ${opts.version}`;
	const output = await $`git log --oneline "--grep=${commitMessage}"`.text();
	console.log(output);
	if (output.trim() !== "") {
		$.logLight("Release please version already configured");
	} else {
		$.logStep("Updating Release Please", commitMessage);
		await $`git commit --allow-empty -m "${commitMessage}" -m "Release-As: ${opts.version}"`;
	}

	// Push changes
	await $`git push`;
}

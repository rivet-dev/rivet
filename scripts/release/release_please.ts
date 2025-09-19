import type { ReleaseOpts } from "./main";
import { $ } from "execa";

export async function configureReleasePlease(opts: ReleaseOpts) {
	// Check if the Release-As commit already exists
	const commitMessage = `chore: release ${opts.version}`;
	console.log(`==> Updating Release Please: ${commitMessage}`);
	await $({ shell: true })`git commit --allow-empty -m "${commitMessage}" -m "Release-As: ${opts.version}"`;
}

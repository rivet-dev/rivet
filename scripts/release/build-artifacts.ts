import * as path from "node:path";
import { $ } from "execa";
import type { ReleaseOpts } from "./main";
import { assertDirExists, uploadDirToReleases } from "./utils";

export async function buildJsArtifacts(opts: ReleaseOpts) {
	await buildAndUploadDevtools(opts);
}

async function buildAndUploadDevtools(opts: ReleaseOpts) {
	console.log(`==> Building DevTools`);

	// Build devtools package
	await $({
		stdio: "inherit",
		cwd: opts.root,
	})`pnpm build -F @rivetkit/devtools`;

	console.log(`✅ DevTools built successfully`);

	// Upload devtools to R2
	console.log(`==> Uploading DevTools Artifacts`);

	const devtoolsDistPath = path.resolve(
		opts.root,
		"rivetkit-typescript/packages/devtools/dist",
	);

	await assertDirExists(devtoolsDistPath);

	// Upload to commit directory
	console.log(`Uploading devtools to rivet/${opts.commit}/devtools/`);
	await uploadDirToReleases(devtoolsDistPath, `rivet/${opts.commit}/devtools/`);

	console.log(`✅ DevTools artifacts uploaded successfully`);
}

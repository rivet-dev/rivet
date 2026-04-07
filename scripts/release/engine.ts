import * as fs from "node:fs/promises";
import * as os from "node:os";
import * as path from "node:path";
import { $ } from "execa";
import type { ReleaseOpts } from "./main";
import {
	assert,
	downloadFileFromReleases,
	fetchGitRef,
	versionOrCommitToRef,
} from "./utils";

interface EnginePlatformPackage {
	dirName: string;
	packageName: string;
	sourceArtifactName: string;
	targetBinaryName: string;
}

const ENGINE_PLATFORM_PACKAGES: EnginePlatformPackage[] = [
	{
		dirName: "darwin-arm64",
		packageName: "@rivetkit/engine-darwin-arm64",
		sourceArtifactName: "rivet-engine-aarch64-apple-darwin",
		targetBinaryName: "rivet-engine",
	},
	{
		dirName: "darwin-x64",
		packageName: "@rivetkit/engine-darwin-x64",
		sourceArtifactName: "rivet-engine-x86_64-apple-darwin",
		targetBinaryName: "rivet-engine",
	},
	{
		dirName: "linux-x64-musl",
		packageName: "@rivetkit/engine-linux-x64-musl",
		sourceArtifactName: "rivet-engine-x86_64-unknown-linux-musl",
		targetBinaryName: "rivet-engine",
	},
	{
		dirName: "win32-x64-gnu",
		packageName: "@rivetkit/engine-win32-x64-gnu",
		sourceArtifactName: "rivet-engine-x86_64-pc-windows-gnu.exe",
		targetBinaryName: "rivet-engine.exe",
	},
];

async function npmVersionExists(
	packageName: string,
	version: string,
): Promise<boolean> {
	try {
		await $({
			stdout: "ignore",
			stderr: "pipe",
		})`npm view ${packageName}@${version} version`;
		return true;
	} catch (error) {
		const stderr =
			typeof error === "object" &&
			error !== null &&
			"stderr" in error &&
			typeof error.stderr === "string"
				? error.stderr
				: "";
		if (
			stderr &&
			!stderr.includes(`No match found for version ${version}`) &&
			!stderr.includes(`'${packageName}@${version}' is not in this registry.`)
		) {
			throw new Error(`unexpected npm view output for ${packageName}: ${stderr}`);
		}
		return false;
	}
}

async function resolveSourcePrefix(opts: ReleaseOpts): Promise<string> {
	if (!opts.reuseEngineVersion) {
		return opts.commit;
	}

	if (opts.reuseEngineVersion.includes(".")) {
		return opts.reuseEngineVersion;
	}

	const ref = versionOrCommitToRef(opts.reuseEngineVersion);
	await fetchGitRef(ref);
	const result = await $`git rev-parse ${ref}`;
	return result.stdout.trim().slice(0, 7);
}

function npmTagForVersion(version: string): string {
	return version.includes("-rc.") ? "rc" : "latest";
}

async function stageAndPublishPlatformPackage(
	opts: ReleaseOpts,
	sourcePrefix: string,
	platformPackage: EnginePlatformPackage,
	tag: string,
): Promise<void> {
	const versionExists = await npmVersionExists(
		platformPackage.packageName,
		opts.version,
	);
	if (versionExists) {
		console.log(
			`Version ${opts.version} of ${platformPackage.packageName} already exists. Skipping...`,
		);
		return;
	}

	const templateDir = path.join(
		opts.root,
		"rivetkit-typescript/packages/engine/npm",
		platformPackage.dirName,
	);
	const tmpDir = await fs.mkdtemp(
		path.join(os.tmpdir(), `rivet-engine-${platformPackage.dirName}-`),
	);

	try {
		await fs.copyFile(
			path.join(templateDir, "package.json"),
			path.join(tmpDir, "package.json"),
		);

		const remotePath = `rivet/${sourcePrefix}/engine/${platformPackage.sourceArtifactName}`;
		const localBinaryPath = path.join(tmpDir, platformPackage.targetBinaryName);
		console.log(
			`==> Downloading ${platformPackage.packageName} binary from ${remotePath}`,
		);
		await downloadFileFromReleases(remotePath, localBinaryPath);

		if (!platformPackage.targetBinaryName.endsWith(".exe")) {
			await fs.chmod(localBinaryPath, 0o755);
		}

		console.log(
			`==> Publishing to NPM: ${platformPackage.packageName}@${opts.version}`,
		);
		await $({
			cwd: tmpDir,
			stdio: "inherit",
		})`npm publish --access public --tag ${tag}`;
	} finally {
		await fs.rm(tmpDir, { recursive: true, force: true });
	}
}

export async function publishEnginePackage(opts: ReleaseOpts) {
	const sourcePrefix = await resolveSourcePrefix(opts);
	const tag = npmTagForVersion(opts.version);

	for (const platformPackage of ENGINE_PLATFORM_PACKAGES) {
		await stageAndPublishPlatformPackage(opts, sourcePrefix, platformPackage, tag);
	}

	const metaPackageName = "@rivetkit/engine";
	const metaVersionExists = await npmVersionExists(metaPackageName, opts.version);
	if (metaVersionExists) {
		console.log(`Version ${opts.version} of ${metaPackageName} already exists. Skipping...`);
		return;
	}

	const metaPackageDir = path.join(opts.root, "rivetkit-typescript/packages/engine");
	const packageJson = JSON.parse(
		await fs.readFile(path.join(metaPackageDir, "package.json"), "utf-8"),
	);

	assert(
		packageJson.optionalDependencies &&
			typeof packageJson.optionalDependencies === "object",
		`${metaPackageName} must define optionalDependencies`,
	);

	console.log(`==> Publishing to NPM: ${metaPackageName}@${opts.version}`);
	await $({
		cwd: opts.root,
		stdio: "inherit",
	})`pnpm --filter ${metaPackageName} publish --access public --tag ${tag} --no-git-checks`;
}

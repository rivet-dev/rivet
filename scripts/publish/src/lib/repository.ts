import { existsSync } from "node:fs";
import { join } from "node:path";
import { $ } from "execa";

/** The public repository that publish.yaml runs in and that release tags live on. */
export const OSS_REPOSITORY = "rivet-dev/rivet";

/** The private repository that is the source of truth for every release. */
export const EE_REPOSITORY = "rivet-dev/engine-ee";

/** Whether `remote` is a plain GitHub URL for `repository`, in any common Git URL form. */
export function isGitHubRemote(remote: string, repository: string): boolean {
	const value = remote.trim();
	const [owner, name] = repository.split("/");
	const path = new RegExp(`^/${owner}/${name}(?:\\.git)?/?$`);
	if (new RegExp(`^git@github\\.com:${owner}/${name}(?:\\.git)?$`).test(value)) return true;
	try {
		const url = new URL(value);
		return (
			(url.protocol === "https:" || url.protocol === "ssh:") &&
			url.hostname === "github.com" &&
			!url.port &&
			!url.password &&
			(url.username === "" || (url.protocol === "ssh:" && url.username === "git")) &&
			path.test(url.pathname) &&
			!url.search &&
			!url.hash
		);
	} catch {
		return false;
	}
}

export async function originRemoteUrl(repoRoot: string): Promise<string> {
	if (existsSync(join(repoRoot, ".git"))) {
		try {
			return (await $({ cwd: repoRoot })`git remote get-url origin`).stdout.trim();
		} catch {
			// A colocated jj checkout can still have the remote in its Git store.
		}
	}
	try {
		const { stdout } = await $({ cwd: repoRoot })`jj git remote list`;
		const origin = stdout.split("\n").find((line) => line.startsWith("origin "));
		if (origin) return origin.slice("origin ".length).trim();
	} catch {
		// Report the same clear error for missing Git, jj, and origin remotes.
	}
	throw new Error(`release requires an origin remote for github.com/${EE_REPOSITORY}`);
}

/**
 * Releases are cut from EE, which publishes to OSS through the snapshot sync. Running the
 * release from an OSS checkout would commit to OSS directly and fork it from EE.
 */
export async function requireEeReleaseRepository(repoRoot: string): Promise<void> {
	const origin = await originRemoteUrl(repoRoot);
	if (!isGitHubRemote(origin, EE_REPOSITORY)) {
		throw new Error(
			`release is only allowed from a github.com/${EE_REPOSITORY} jj workspace; origin is ${origin}`,
		);
	}
}

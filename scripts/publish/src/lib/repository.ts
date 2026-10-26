import { existsSync } from "node:fs";
import { join } from "node:path";
import { $ } from "execa";

/** The local counterpart of publish.yaml's rivet-dev/rivet repository-ID guard. */
export function isOssRepositoryRemote(remote: string): boolean {
	const value = remote.trim();
	if (/^git@github\.com:rivet-dev\/rivet(?:\.git)?$/.test(value)) return true;
	try {
		const url = new URL(value);
		return (
			(url.protocol === "https:" || url.protocol === "ssh:") &&
			url.hostname === "github.com" &&
			!url.port &&
			!url.password &&
			(url.username === "" || (url.protocol === "ssh:" && url.username === "git")) &&
			/^\/rivet-dev\/rivet(?:\.git)?\/?$/.test(url.pathname) &&
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
	throw new Error("release requires an origin remote for github.com/rivet-dev/rivet");
}

export async function requireOssReleaseRepository(repoRoot: string): Promise<void> {
	const origin = await originRemoteUrl(repoRoot);
	if (!isOssRepositoryRemote(origin)) {
		throw new Error(
			`release is only allowed from github.com/rivet-dev/rivet (repository ID 654560183); origin is ${origin}`,
		);
	}
}

// Builds a small, self-contained dependency closure for the Pi agent at
// `.agent-modules/`, which the example mounts as the VM's `/root/node_modules`
// via `nodeModulesMount(".agent-modules/node_modules")` in src/server.ts.
//
// Why a dedicated dir instead of the example's own node_modules: in this pnpm
// monorepo the example's deps are symlinks that resolve out to the workspace
// root store, and the VM resolver (correctly) refuses symlinks that escape the
// mounted root. Mounting the workspace root itself would expose the entire
// ~4.5 GB monorepo to the VM. A flat `npm install` here gives the agent exactly
// its own closure and nothing else.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const dir = join(root, ".agent-modules");
const pi = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const PI_VER = pi.dependencies["@rivet-dev/agent-os-pi"];
// Keep agent + SDK versions in sync with the example's pinned agent-os version.
const deps = {
	"@rivet-dev/agent-os-pi": PI_VER,
	"@rivet-dev/agent-os-core": PI_VER,
	"@mariozechner/pi-coding-agent": "0.60.0",
};

const stamp = join(dir, ".deps.json");
const want = JSON.stringify(deps);
if (existsSync(stamp) && readFileSync(stamp, "utf8") === want) {
	console.log("[prepare-agent-modules] up to date");
	process.exit(0);
}
mkdirSync(dir, { recursive: true });
writeFileSync(join(dir, "package.json"), JSON.stringify({ name: "agent-modules", private: true, dependencies: deps }, null, 2));
console.log("[prepare-agent-modules] installing agent closure into .agent-modules ...");
execFileSync("npm", ["install", "--no-audit", "--no-fund", "--loglevel=error"], { cwd: dir, stdio: "inherit" });
writeFileSync(stamp, want);
console.log("[prepare-agent-modules] done");

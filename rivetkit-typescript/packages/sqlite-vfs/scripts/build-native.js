import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve, delimiter } from "node:path";
import { fileURLToPath } from "node:url";
import { arch, platform } from "node:process";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../../..");
const rustRoot = resolve(repoRoot, "rivetkit-rust");
const targetDir = resolve(rustRoot, "target", "debug");

const candidates = [];
if (process.env.CARGO) {
	candidates.push(process.env.CARGO);
}
if (process.env.HOME) {
	candidates.push(resolve(process.env.HOME, ".cargo", "bin", "cargo"));
}
const pathEntries = (process.env.PATH ?? "").split(delimiter);
for (const entry of pathEntries) {
	if (entry) {
		candidates.push(resolve(entry, "cargo"));
	}
}

const cargoPath = candidates.find((candidate) => existsSync(candidate));
if (!cargoPath) {
	throw new Error("cargo not found in PATH or default locations");
}

execFileSync(cargoPath, ["build", "-p", "rivetkit-sqlite-vfs-native"], {
	cwd: rustRoot,
	stdio: "inherit",
});

let libName = "librivetkit_sqlite_vfs_native.so";
if (platform === "darwin") {
	libName = "librivetkit_sqlite_vfs_native.dylib";
} else if (platform === "win32") {
	libName = "rivetkit_sqlite_vfs_native.dll";
}

const packageDir = resolve(
	repoRoot,
	"rivetkit-typescript",
	"packages",
	`sqlite-vfs-${platform}-${arch}`,
);
const destDir = join(packageDir, "bin");
const destFile = join(destDir, "rivetkit_sqlite_vfs_native.node");

mkdirSync(destDir, { recursive: true });
copyFileSync(join(targetDir, libName), destFile);

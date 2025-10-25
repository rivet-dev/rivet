import type { ActorKey } from "@/actor/mod";
import {
	getNodeCrypto,
	getNodeFs,
	getNodeFsSync,
	getNodeOs,
	getNodePath,
} from "@/utils/node";

/**
 * Generate a deterministic actor ID from name and key
 */
export function generateActorId(name: string, key: ActorKey): string {
	// Generate deterministic key string
	const jsonString = JSON.stringify([name, key]);

	// Hash to ensure safe file system names
	const crypto = getNodeCrypto();
	const hash = crypto
		.createHash("sha256")
		.update(jsonString)
		.digest("hex")
		.substring(0, 16);

	return hash;
}

/**
 * Create a hash for a path, normalizing it first
 */
function createHashForPath(dirPath: string): string {
	const path = getNodePath();
	// Normalize the path first
	const normalizedPath = path.normalize(dirPath);

	// Extract the last path component for readability
	const lastComponent = path.basename(normalizedPath);

	// Create SHA-256 hash
	const crypto = getNodeCrypto();
	const hash = crypto
		.createHash("sha256")
		.update(normalizedPath)
		.digest("hex")
		.substring(0, 8); // Take first 8 characters for brevity

	return `${lastComponent}-${hash}`;
}

/**
 * Get the storage path for the current working directory or a specified path
 */
export function getStoragePath(customPath?: string): string {
	const dataPath = getDataPath("rivetkit");
	const pathToHash = customPath || process.cwd();
	const dirHash = createHashForPath(pathToHash);
	const path = getNodePath();
	return path.join(dataPath, dirHash);
}

/**
 * Check if a path exists
 */
export async function pathExists(path: string): Promise<boolean> {
	try {
		const fs = getNodeFs();
		await fs.access(path);
		return true;
	} catch {
		return false;
	}
}

/**
 * Ensure a directory exists, creating it if necessary
 */
export async function ensureDirectoryExists(
	directoryPath: string,
): Promise<void> {
	if (!(await pathExists(directoryPath))) {
		const fs = getNodeFs();
		await fs.mkdir(directoryPath, { recursive: true });
	}
}

/**
 * Ensure a directory exists synchronously - only used during initialization
 * All other operations use the async version
 */
export function ensureDirectoryExistsSync(directoryPath: string): void {
	const fsSync = getNodeFsSync();
	if (!fsSync.existsSync(directoryPath)) {
		fsSync.mkdirSync(directoryPath, { recursive: true });
	}
}

/**
 * Returns platform-specific data directory
 */
function getDataPath(appName: string): string {
	const platform = process.platform;
	const os = getNodeOs();
	const homeDir = os.homedir();
	const path = getNodePath();

	switch (platform) {
		case "win32":
			return path.join(
				process.env.APPDATA || path.join(homeDir, "AppData", "Roaming"),
				appName,
			);
		case "darwin":
			return path.join(
				homeDir,
				"Library",
				"Application Support",
				appName,
			);
		default: // linux and others
			return path.join(
				process.env.XDG_DATA_HOME ||
					path.join(homeDir, ".local", "share"),
				appName,
			);
	}
}

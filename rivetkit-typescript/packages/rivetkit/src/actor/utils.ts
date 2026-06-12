/**
 * Checks if a path is an actor state path within the persisted actor data.
 */
export function isStatePath(path: string): boolean {
	return path === "state" || path.startsWith("state.");
}

/**
 * Checks if a path is a connection state path within the persisted actor data.
 */
export function isConnStatePath(path: string): boolean {
	if (!path.startsWith("connections.")) {
		return false;
	}
	const stateIndex = path.indexOf(".state", 12); // Start after "connections."
	if (stateIndex === -1) {
		return false;
	}
	const afterState = stateIndex + 6; // ".state".length = 6
	// Check if ".state" is followed by end of string or "."
	return path.length === afterState || path[afterState] === ".";
}

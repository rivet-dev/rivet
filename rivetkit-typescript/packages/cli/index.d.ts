/** Returns the absolute path to the rivet CLI binary for the current host. */
export function getCliPath(): string;

/** Returns the expected platform-specific npm package for the current host, or null if unsupported. */
export function getPlatformPackageName(): string | null;

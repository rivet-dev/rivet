/**
 * Returns the absolute path to the rivet-engine binary for the current host.
 *
 * Resolution order:
 *   1. `RIVET_ENGINE_BINARY` env var (absolute path override)
 *   2. Local `rivet-engine` binary next to this package (dev builds)
 *   3. The platform-specific `@rivetkit/engine-cli-<platform>` npm package
 *
 * Throws if none of the above yields a binary.
 */
export function getEnginePath(): string;

/** Returns the expected name of the platform-specific npm package for the current host, or null if unsupported. */
export function getPlatformPackageName(): string | null;

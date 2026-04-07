export interface ResolvedEngineBinary {
	packageName: string;
	packageDir: string;
	binaryPath: string;
	version: string;
}

export declare function getInstalledVersion(): string;
export declare function getEnginePackageNameForPlatform(
	platform?: NodeJS.Platform,
	arch?: typeof process.arch,
): string;
export declare function resolveEngineBinaryFor(
	platform?: NodeJS.Platform,
	arch?: typeof process.arch,
): ResolvedEngineBinary;
export declare function resolveEngineBinary(): ResolvedEngineBinary;

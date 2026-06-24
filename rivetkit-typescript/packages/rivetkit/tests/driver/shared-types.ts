import type { Encoding } from "../../src/client/mod";

export type DriverRuntime = "native" | "wasm";
export type DriverSqliteBackend = "local" | "remote";

export interface SkipTests {
	schedule?: boolean;
	sleep?: boolean;
	hibernation?: boolean;
}

export interface DriverTestFeatures {
	hibernatableWebSocketProtocol?: boolean;
}

export interface DriverDeployOutput {
	endpoint: string;
	namespace: string;
	runnerName: string;
	hardCrashActor?: (actorId: string) => Promise<void>;
	hardCrashPreservesData?: boolean;
	getRuntimeOutput?: () => string;
	cleanup(): Promise<void>;
}

export interface DriverTestConfig {
	start(): Promise<DriverDeployOutput>;
	runtime: DriverRuntime;
	sqliteBackend: DriverSqliteBackend;
	useRealTimers?: boolean;
	HACK_skipCleanupNet?: boolean;
	skip?: SkipTests;
	features?: DriverTestFeatures;
	encodings?: Encoding[];
	encoding?: Encoding;
	cleanup?: () => Promise<void>;
}

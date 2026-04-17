import type { Encoding } from "../../src/client/mod";

export interface SkipTests {
	schedule?: boolean;
	sleep?: boolean;
	hibernation?: boolean;
	agentOs?: boolean;
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
	cleanup(): Promise<void>;
}

export interface DriverTestConfig {
	start(): Promise<DriverDeployOutput>;
	useRealTimers?: boolean;
	HACK_skipCleanupNet?: boolean;
	skip?: SkipTests;
	features?: DriverTestFeatures;
	encodings?: Encoding[];
	encoding?: Encoding;
	cleanup?: () => Promise<void>;
}

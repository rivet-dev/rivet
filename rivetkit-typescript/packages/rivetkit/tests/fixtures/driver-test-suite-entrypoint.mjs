import { isMainThread } from "node:worker_threads";
import { register } from "tsx/esm/api";

// The --import tsx preload only registers in the main thread in older tsx
// releases. Register explicitly before loading the same fixture in a worker.
if (!isMainThread) register();
await import("./driver-test-suite-runtime.ts");

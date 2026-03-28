import {
	type UnboundedReceiver,
	type UnboundedSender,
	unboundedChannel,
} from "antiox/sync/mpsc";
import { spawn } from "antiox/task";
import { injectLatency } from "./utils.js";

export type LatencyChannel<T> = [UnboundedSender<T>, UnboundedReceiver<T>];

/**
 * Returns an antiox channel that delays delivery to the receiver by the
 * configured latency while preserving message order.
 */
export function latencyChannel<T>(debugLatencyMs?: number): LatencyChannel<T> {
	if (!debugLatencyMs) {
		return unboundedChannel<T>();
	}

	const [inputTx, inputRx] = unboundedChannel<T>();
	const [outputTx, outputRx] = unboundedChannel<T>();

	spawn(async () => {
		for await (const message of inputRx) {
			await injectLatency(debugLatencyMs);

			try {
				outputTx.send(message);
			} catch {
				inputRx.close();
				break;
			}
		}

		outputTx.close();
	});

	return [inputTx, outputRx];
}

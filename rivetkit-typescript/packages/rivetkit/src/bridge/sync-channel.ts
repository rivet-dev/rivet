import { decode as cborDecode, encode as cborEncode } from "cbor-x";
import {
	type BridgeErrorPayload,
	SYNC_DEFAULT_BUFFER_BYTES,
	SYNC_HEADER_BYTES,
	SYNC_STATUS_ERROR,
	SYNC_STATUS_OK,
	SYNC_STATUS_OVERFLOW,
	SYNC_STATUS_PENDING,
} from "./protocol";

/**
 * Blocking synchronous RPC channel for the bridge child.
 *
 * The child posts an rpc:sync message carrying a SharedArrayBuffer, then parks
 * on `Atomics.wait` until the host writes the cbor-encoded result and flips
 * the status word. The host never blocks on the child, so this cannot
 * deadlock. Used only for the cold synchronous CoreRuntime reads listed in
 * BRIDGE_SYNC_METHODS.
 */

export interface SyncCallResult {
	ok: boolean;
	value?: unknown;
	error?: BridgeErrorPayload;
}

export class SyncCaller {
	#sab: SharedArrayBuffer;

	constructor(initialBytes = SYNC_DEFAULT_BUFFER_BYTES) {
		this.#sab = new SharedArrayBuffer(initialBytes);
	}

	/**
	 * Run one blocking call. `post` must enqueue an rpc:sync message carrying
	 * the provided SAB to the host; it may run more than once when the
	 * response overflows the current buffer.
	 */
	call(post: (sab: SharedArrayBuffer) => void): SyncCallResult {
		for (;;) {
			const status = new Int32Array(this.#sab, 0, 2);
			Atomics.store(status, 0, SYNC_STATUS_PENDING);
			post(this.#sab);
			Atomics.wait(status, 0, SYNC_STATUS_PENDING);

			const code = Atomics.load(status, 0);
			const byteLength = Atomics.load(status, 1);
			if (code === SYNC_STATUS_OVERFLOW) {
				// The response did not fit; grow and retry with a fresh buffer.
				this.#sab = new SharedArrayBuffer(
					SYNC_HEADER_BYTES + byteLength,
				);
				continue;
			}

			// Copy out of the SAB before decoding; cbor-x cannot decode views
			// over shared memory.
			const payload = new Uint8Array(byteLength);
			payload.set(
				new Uint8Array(this.#sab, SYNC_HEADER_BYTES, byteLength),
			);
			const value = byteLength > 0 ? cborDecode(payload) : undefined;
			if (code === SYNC_STATUS_ERROR) {
				return { ok: false, error: value as BridgeErrorPayload };
			}
			return { ok: true, value };
		}
	}
}

/** Host side: write a result (or error) into the child's SAB and wake it. */
export function respondSync(sab: SharedArrayBuffer, result: SyncCallResult) {
	const status = new Int32Array(sab, 0, 2);
	const payload = cborEncode(
		result.ok ? (result.value ?? null) : result.error,
	) as Uint8Array;
	// `value === undefined` encodes to a payload, so null-coalesce above keeps
	// byteLength meaningful; undefined results decode as null and callers in
	// the child treat null and undefined uniformly.

	if (payload.byteLength > sab.byteLength - SYNC_HEADER_BYTES) {
		Atomics.store(status, 1, payload.byteLength);
		Atomics.store(status, 0, SYNC_STATUS_OVERFLOW);
		Atomics.notify(status, 0);
		return;
	}

	new Uint8Array(sab, SYNC_HEADER_BYTES, payload.byteLength).set(payload);
	Atomics.store(status, 1, payload.byteLength);
	Atomics.store(status, 0, result.ok ? SYNC_STATUS_OK : SYNC_STATUS_ERROR);
	Atomics.notify(status, 0);
}

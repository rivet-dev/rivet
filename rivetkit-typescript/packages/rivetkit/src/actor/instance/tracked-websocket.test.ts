import { describe, expect, test, vi } from "vitest";
import type {
	RivetEvent,
	UniversalWebSocket,
} from "@/common/websocket-interface";
import { TrackedWebSocket } from "./tracked-websocket";

class MockWebSocket implements UniversalWebSocket {
	readonly CONNECTING = 0 as const;
	readonly OPEN = 1 as const;
	readonly CLOSING = 2 as const;
	readonly CLOSED = 3 as const;

	readyState: 0 | 1 | 2 | 3 = this.OPEN;
	binaryType: "arraybuffer" | "blob" = "arraybuffer";
	bufferedAmount = 0;
	extensions = "";
	protocol = "";
	url = "ws://example.test";

	#listeners = new Map<string, Array<(event: any) => void | Promise<void>>>();

	send(_data: string | ArrayBufferLike | Blob | ArrayBufferView): void {}

	close(_code?: number, _reason?: string): void {}

	addEventListener(
		type: string,
		listener: (event: any) => void | Promise<void>,
	): void {
		if (!this.#listeners.has(type)) {
			this.#listeners.set(type, []);
		}

		this.#listeners.get(type)!.push(listener);
	}

	removeEventListener(
		type: string,
		listener: (event: any) => void | Promise<void>,
	): void {
		const listeners = this.#listeners.get(type);
		if (!listeners) return;

		const index = listeners.indexOf(listener);
		if (index >= 0) listeners.splice(index, 1);
	}

	dispatchEvent(event: RivetEvent): boolean {
		for (const listener of this.#listeners.get(event.type) ?? []) {
			void listener(event);
		}

		return true;
	}

	onopen = null;
	onclose = null;
	onerror = null;
	onmessage = null;
}

describe("TrackedWebSocket", () => {
	test("does not synthesize open events", async () => {
		const inner = new MockWebSocket();
		const tracked = new TrackedWebSocket(inner, {
			onPromise: vi.fn(),
			onError: vi.fn(),
		});
		const onOpen = vi.fn();

		tracked.addEventListener("open", onOpen);
		await Promise.resolve();

		expect(onOpen).not.toHaveBeenCalled();
	});

	test("forwards real open events from the inner websocket", async () => {
		const inner = new MockWebSocket();
		const onPromise = vi.fn();
		const tracked = new TrackedWebSocket(inner, {
			onPromise,
			onError: vi.fn(),
		});

		const onOpen = vi.fn(async () => {});
		tracked.onopen = onOpen;

		inner.dispatchEvent({ type: "open" });
		await Promise.resolve();

		expect(onOpen).toHaveBeenCalledTimes(1);
		expect(onPromise).toHaveBeenCalledTimes(1);
		expect(onPromise).toHaveBeenCalledWith("open", expect.any(Promise));
	});
});

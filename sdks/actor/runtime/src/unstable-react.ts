// @ts-ignore we do not have types for this lib
import { renderToPipeableStream } from "@jogit/tmp-react-server-dom-nodeless";
import getStream from "get-stream";
import { isValidElement } from "react";
import { Actor } from "./actor";

/**
 * A React Server Components (RSC) actor.
 *
 * Supports rendering React elements as RPC responses.
 *
 * @see [Documentation](https://rivet.gg/docs/client/react)
 * @experimental
 */
export class RscActor<
	State = undefined,
	ConnParams = undefined,
	ConnState = undefined,
> extends Actor<State, ConnParams, ConnState> {
	/**
	 * Updates the RSCs for all connected clients.
	 */
	public _updateRsc() {
		// Broadcast a message to all connected clients, telling them to re-render
		this._broadcast("__rsc");
	}

	/**
	 * Overrides default behavior to update the RSCs when the state changes.
	 * @private
	 * @internal
	 */
	override _onStateChange() {
		this._updateRsc();
	}

	/**
	 * Overrides default behavior to render React elements as RSC response.
	 * @private
	 * @internal
	 */
	protected override _onBeforeRpcResponse<Out>(
		_name: string,
		_args: unknown[],
		output: Out,
	): Out {
		if (!isValidElement(output)) {
			return super._onBeforeRpcResponse(_name, _args, output);
		}

		// The output is a React element, so we need to transform it into a valid rsc message
		const { readable, ...writable } = nodeStreamToWebStream();

		const stream = renderToPipeableStream(output);

		stream.pipe(writable);

		return getStream(readable) as Out;
	}
}

function nodeStreamToWebStream() {
	const buffer: Uint8Array[] = [];
	let writer: WritableStreamDefaultWriter<Uint8Array> | null = null;

	const writable = new WritableStream<Uint8Array>({
		write(chunk) {
			buffer.push(chunk);
		},
		close() {},
	});

	const readable = new ReadableStream<Uint8Array>({
		start() {},
		async pull(controller) {
			if (buffer.length > 0) {
				const chunk = buffer.shift(); // Get the next chunk from the buffer
				if (chunk) {
					controller.enqueue(chunk); // Push it to the readable stream
				}
			} else {
				if (writable.locked) {
					await new Promise((resolve) => setTimeout(resolve, 10));
					return this.pull?.(controller);
				}
				return controller.close();
			}
		},
		cancel() {},
	});

	return {
		readable,
		on: (str: string, fn: () => void) => {
			if (str === "drain") {
				writer = writable.getWriter();
				fn();
			}
		},
		write(chunk: Uint8Array) {
			writer?.write(chunk);
		},
		flush() {
			writer?.close();
		},
		end() {
			writer?.releaseLock();
		},
	};
}

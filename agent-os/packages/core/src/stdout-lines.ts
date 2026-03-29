// Helper to convert onStdout callback bytes into an AsyncIterable<string> of lines

interface StdoutLineIterable {
	/** Pass this as the onStdout callback to kernel.spawn(). */
	onStdout: (data: Uint8Array) => void;
	/** Async iterable of newline-delimited stdout lines. */
	iterable: AsyncIterable<string>;
}

/**
 * Creates a bridge between the spawn onStdout callback and the
 * AsyncIterable<string> expected by AcpClient.
 *
 * Bytes arriving via onStdout are buffered and split on newlines.
 * Complete lines are pushed to an async queue consumed by the iterable.
 */
export function createStdoutLineIterable(): StdoutLineIterable {
	let buffer = "";
	const queue: string[] = [];
	let resolve: (() => void) | null = null;
	let done = false;

	const onStdout = (data: Uint8Array): void => {
		if (done) return;

		buffer += new TextDecoder().decode(data);
		const lines = buffer.split("\n");
		// Keep the last (potentially incomplete) chunk in the buffer
		buffer = lines.pop() ?? "";

		for (const line of lines) {
			queue.push(line);
			if (resolve) {
				resolve();
				resolve = null;
			}
		}
	};

	const iterable: AsyncIterable<string> = {
		[Symbol.asyncIterator]() {
			return {
				async next(): Promise<IteratorResult<string>> {
					while (queue.length === 0) {
						if (done) return { value: undefined, done: true };
						await new Promise<void>((r) => {
							resolve = r;
						});
					}
					const value = queue.shift() as string;
					return { value, done: false };
				},
				return(): Promise<IteratorResult<string>> {
					done = true;
					if (resolve) {
						resolve();
						resolve = null;
					}
					return Promise.resolve({ value: undefined, done: true });
				},
			};
		},
	};

	return { onStdout, iterable };
}

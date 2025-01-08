export async function safeAsync<T>(
	fn: Promise<T>,
): Promise<[T, undefined] | [undefined, unknown]> {
	try {
		return [await fn, undefined];
	} catch (e) {
		return [undefined, e];
	}
}

export const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export async function waitFor<T>(fn: () => Promise<T | null | undefined>, timeoutMs = 5000): Promise<T> {
	const start = Date.now();
	while (Date.now() - start < timeoutMs) {
		const value = await fn();
		if (value != null) return value;
		await sleep(25);
	}
	throw new Error("timed out waiting for value");
}

export function nowTime() {
	return new Date().toLocaleTimeString();
}

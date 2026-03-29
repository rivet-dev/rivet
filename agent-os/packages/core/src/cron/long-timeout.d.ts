declare module "long-timeout" {
	interface LongTimeout {
		close(): void;
		ref(): void;
		unref(): void;
	}

	function setTimeout(fn: () => void, delay: number): LongTimeout;
	function clearTimeout(timer: LongTimeout): void;
	function setInterval(fn: () => void, delay: number): LongTimeout;
	function clearInterval(timer: LongTimeout): void;
}

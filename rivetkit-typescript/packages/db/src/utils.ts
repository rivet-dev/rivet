/**
 * On serverless environments, we use a shim, as not all methods are available.
 * This is a minimal shim that only includes the `exec` method, which is used for
 * running raw SQL commands.
 */
export type SQLiteShim = {
	exec: (query: string, ...args: unknown[]) => unknown[];
};

export function isSQLiteShim<T>(conn: unknown): conn is SQLiteShim & T {
	return (
		typeof conn === "object" &&
		conn !== null &&
		"exec" in conn &&
		typeof (conn as SQLiteShim).exec === "function"
	);
}

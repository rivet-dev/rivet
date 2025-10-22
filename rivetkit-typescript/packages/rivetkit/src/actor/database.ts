export type InferDatabaseClient<DBProvider extends AnyDatabaseProvider> =
	DBProvider extends DatabaseProvider<any>
		? Awaited<ReturnType<DBProvider["createClient"]>>
		: never;

export type AnyDatabaseProvider = DatabaseProvider<any> | undefined;

export type DatabaseProvider<DB extends { execute: (query: string) => any }> = {
	/**
	 * Creates a new database client for the actor.
	 * The result is passed to the actor context as `c.db`.
	 * @experimental
	 */
	createClient: (ctx: {
		getDatabase: () => Promise<string | unknown>;
	}) => Promise<DB>;
	/**
	 * Runs before the actor has started.
	 * Use this to run migrations or other setup tasks.
	 * @experimental
	 */
	onMigrate: (client: DB) => void | Promise<void>;
};

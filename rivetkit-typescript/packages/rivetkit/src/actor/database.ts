import type {
	AnyDatabaseProvider,
	DatabaseProvider,
	DrizzleDatabaseClient,
	RawDatabaseClient,
} from "@/db/config";

export type InferDatabaseClient<DBProvider extends AnyDatabaseProvider> =
	DBProvider extends DatabaseProvider<any>
		? Awaited<ReturnType<DBProvider["createClient"]>>
		: never;

export type {
	AnyDatabaseProvider,
	DatabaseProvider,
	DrizzleDatabaseClient,
	RawDatabaseClient,
};

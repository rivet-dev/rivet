import type { IDatabase } from "@rivetkit/sqlite-vfs";
import type { DatabaseProviderContext } from "./config";
import { openNativeDatabase } from "./native-adapter";
import { nativeSqliteAvailable } from "./native-sqlite";
import { createActorKvStore } from "./shared";

type OpenedKvStore = ReturnType<typeof createActorKvStore>;

export interface OpenedActorDatabase {
	database: IDatabase;
	kvStore?: OpenedKvStore;
}

let nativeFallbackWarned = false;

export async function openActorDatabase(
	ctx: DatabaseProviderContext,
): Promise<OpenedActorDatabase> {
	if (ctx.nativeSqliteConfig && nativeSqliteAvailable()) {
		return {
			database: await openNativeDatabase(
				ctx.actorId,
				ctx.nativeSqliteConfig,
			),
		};
	}

	if (!nativeFallbackWarned) {
		nativeFallbackWarned = true;
		console.warn(
			"native SQLite not available, falling back to WebAssembly. run npm rebuild to install native bindings.",
		);
	}

	if (!ctx.sqliteVfs) {
		throw new Error(
			"SqliteVfs instance not provided in context. The driver must provide a sqliteVfs instance.",
		);
	}

	const kvStore = createActorKvStore(
		ctx.kv,
		ctx.metrics,
		ctx.preloadedEntries,
	);
	return {
		database: await ctx.sqliteVfs.open(ctx.actorId, kvStore),
		kvStore,
	};
}

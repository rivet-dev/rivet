import Database from "better-sqlite3";
import {
	type BetterSQLite3Database,
	drizzle as sqliteDrizzle,
} from "drizzle-orm/better-sqlite3";
import { drizzle as durableDrizzle } from "drizzle-orm/durable-sqlite";
import { migrate as durableMigrate } from "drizzle-orm/durable-sqlite/migrator";
import type { DatabaseProvider, RawAccess } from "@/config";

export * from "drizzle-orm/sqlite-core";

import { type Config, defineConfig as originalDefineConfig } from "drizzle-kit";
import type { SQLiteShim } from "@/utils";

export function defineConfig(
	config: Partial<Config & { driver: "durable-sqlite" }>,
): Config {
	// This is a workaround to avoid the "drizzle-kit" import issue in the examples.
	// It allows us to use the same defineConfig function in both the main package and the examples.
	return originalDefineConfig({
		dialect: "sqlite",
		driver: "durable-sqlite",
		...config,
	});
}

interface DatabaseFactoryConfig<
	TSchema extends Record<string, unknown> = Record<string, never>,
> {
	/**
	 * The database schema.
	 */
	schema?: TSchema;
	migrations?: any;
}

export function db<
	TSchema extends Record<string, unknown> = Record<string, never>,
>(
	config?: DatabaseFactoryConfig<TSchema>,
): DatabaseProvider<BetterSQLite3Database<TSchema> & RawAccess> {
	return {
		createClient: async (ctx) => {
			// Create a database connection using the provided context
			if (!ctx.getDatabase) {
				throw new Error(
					"createDatabase method is not available in context.",
				);
			}

			const conn = await ctx.getDatabase();

			if (!conn) {
				throw new Error(
					"Cannot create database connection, or database feature is not enabled.",
				);
			}

			if (isSQLiteShim(conn)) {
				// If the connection is already an object with exec method, return it
				// i.e. in serverless environments (Cloudflare Workers)
				const client = durableDrizzle<TSchema, SQLiteShim>(
					conn,
					config,
				);
				return Object.assign(client, {
					// client.$client.exec is the underlying SQLite client
					execute: async (query, ...args) =>
						client.$client.exec(query, ...args),
				} satisfies RawAccess);
			}

			// Create a database client using the connection
			const client = sqliteDrizzle({
				client: new Database(conn as string),
				...config,
			});

			return Object.assign(client, {
				execute: async (query, ...args) =>
					client.$client.prepare(query).all(...args),
			} satisfies RawAccess);
		},
		onMigrate: async (client) => {
			// Run migrations if provided in the config
			if (config?.migrations) {
				await durableMigrate(client, config?.migrations);
			}
		},
	};
}

function isSQLiteShim(conn: unknown): conn is SQLiteShim {
	return (
		typeof conn === "object" &&
		conn !== null &&
		"exec" in conn &&
		typeof (conn as any).exec === "function"
	);
}

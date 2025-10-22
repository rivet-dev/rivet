import SQLite from "better-sqlite3";
import type { DatabaseProvider, RawAccess } from "./config";
import { isSQLiteShim, type SQLiteShim } from "./utils";

interface DatabaseFactoryConfig {
	onMigrate?: (db: RawAccess) => Promise<void> | void;
}

export function db({
	onMigrate,
}: DatabaseFactoryConfig = {}): DatabaseProvider<RawAccess> {
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
				return Object.assign({}, conn, {
					execute: async (query, ...args) => {
						return conn.exec(query, ...args);
					},
				} satisfies RawAccess) as SQLiteShim & RawAccess;
			}

			const client = new SQLite(conn as string);
			return Object.assign({}, client, {
				execute: async (query, ...args) => {
					return client.prepare(query).all(...args);
				},
			} satisfies RawAccess) as RawAccess;
		},
		onMigrate: async (client) => {
			// Run migrations if provided in the config
			if (onMigrate) {
				await onMigrate(client);
			}
		},
	};
}

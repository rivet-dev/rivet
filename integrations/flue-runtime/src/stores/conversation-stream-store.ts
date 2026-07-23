import {
	defineSqlConversationStreamStore,
	type ConversationStreamStore,
	type SqlConversationDialect,
	type SqlConversationDialectTx,
} from '@flue/runtime/adapter-kit';
import type { AsyncSqlDb, AsyncSqlValue } from './async-db.js';
import { ListenerRegistry } from './listener-registry.js';

const listeners = new ListenerRegistry();

export function createAsyncConversationStreamStore(
	db: AsyncSqlDb,
	notificationScope = 'default',
): ConversationStreamStore {
	const notificationPath = (path: string) => `${notificationScope}\0${path}`;
	const dialect: SqlConversationDialect = {
		placeholder: () => '?',
		lockClause: '',
		insertIgnorePrefix: 'INSERT OR IGNORE',
		insertIgnoreSuffix: '',
		supportsReturning: true,
		query: (sql: string, params: readonly unknown[]) =>
			db.query(sql, params as readonly AsyncSqlValue[]),
		transaction: <T>(fn: (tx: SqlConversationDialectTx) => Promise<T>) => db.transaction((tx) => fn({
			query: (sql: string, params: readonly unknown[]) =>
				tx.query(sql, params as readonly AsyncSqlValue[]),
		})),
	};
	const store = defineSqlConversationStreamStore(dialect);
	return {
		createStream: (path, identity) => store.createStream(path, identity),
		acquireProducer: (path, producerId) => store.acquireProducer(path, producerId),
		async append(input) {
			const result = await store.append(input);
			listeners.notify(notificationPath(input.path));
			return result;
		},
		read: (path, options) => store.read(path, options),
		getMeta: (path) => store.getMeta(path),
		async delete(path) {
			await store.delete(path);
			listeners.notify(notificationPath(path));
		},
		subscribe: (path, listener) =>
			listeners.subscribe(notificationPath(path), listener),
	};
}

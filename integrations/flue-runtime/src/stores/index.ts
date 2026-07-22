import type { PersistenceStores } from '@flue/runtime/adapter-kit';
import type { AsyncSqlDb } from './async-db.js';
import { createAsyncEventStreamStore } from './event-stream-store.js';
import { createAsyncRunStore } from './run-store.js';
import { createAsyncSessionStore } from './session-store.js';
import { createAsyncSubmissionStore } from './submission-store.js';

export type { AsyncSqlDb, AsyncSqlRow, AsyncSqlRunner, AsyncSqlValue } from './async-db.js';
export { ensureAsyncSqlSchema } from './schema.js';
export { createAsyncEventStreamStore } from './event-stream-store.js';
export { createAsyncRunStore } from './run-store.js';
export { createAsyncSessionStore } from './session-store.js';
export { createAsyncSubmissionStore } from './submission-store.js';

export function createAsyncSqlStores(db: AsyncSqlDb): PersistenceStores {
	return {
		executionStore: {
			sessions: createAsyncSessionStore(db),
			submissions: createAsyncSubmissionStore(db),
		},
		runStore: createAsyncRunStore(db),
		eventStreamStore: createAsyncEventStreamStore(db),
	};
}

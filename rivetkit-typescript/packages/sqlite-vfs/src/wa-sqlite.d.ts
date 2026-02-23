declare module "@rivetkit/sqlite" {
	export function Factory(module: any): any;
}

declare module "@rivetkit/sqlite/src/VFS.js" {
	export class Base {
		handleAsync(fn: () => Promise<number>): number;
	}

	export const SQLITE_OK: number;
	export const SQLITE_CANTOPEN: number;
	export const SQLITE_IOERR_READ: number;
	export const SQLITE_IOERR_SHORT_READ: number;
	export const SQLITE_IOERR_WRITE: number;
	export const SQLITE_IOERR_TRUNCATE: number;
	export const SQLITE_IOERR_FSTAT: number;
	export const SQLITE_OPEN_CREATE: number;
	export const SQLITE_OPEN_READONLY: number;
	export const SQLITE_OPEN_DELETEONCLOSE: number;
	export const SQLITE_OPEN_READWRITE: number;
}

declare module "@rivetkit/sqlite/dist/wa-sqlite-async.mjs" {
	const factory: (config?: { wasmBinary?: ArrayBuffer }) => Promise<any>;
	export default factory;
}

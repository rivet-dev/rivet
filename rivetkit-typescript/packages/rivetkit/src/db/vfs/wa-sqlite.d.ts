declare module "wa-sqlite/src/VFS.js" {
	export const SQLITE_OK: number;
	export const SQLITE_IOERR: number;
	export const SQLITE_IOERR_READ: number;
	export const SQLITE_IOERR_SHORT_READ: number;
	export const SQLITE_IOERR_WRITE: number;
	export const SQLITE_IOERR_TRUNCATE: number;
	export const SQLITE_IOERR_FSTAT: number;
	export const SQLITE_CANTOPEN: number;
	export const SQLITE_OPEN_CREATE: number;
	export const SQLITE_OPEN_READONLY: number;
	export const SQLITE_OPEN_DELETEONCLOSE: number;
	export const SQLITE_NOTFOUND: number;

	/**
	 * Base class for SQLite VFS implementations.
	 * Extend this class and override methods to implement custom file systems.
	 */
	export class Base {
		mxPathName: number;

		/** Close a file */
		xClose(fileId: number): number;

		/** Read data from a file */
		xRead(fileId: number, pData: Uint8Array, iOffset: number): number;

		/** Write data to a file */
		xWrite(fileId: number, pData: Uint8Array, iOffset: number): number;

		/** Truncate a file */
		xTruncate(fileId: number, iSize: number): number;

		/** Sync file data to storage */
		xSync(fileId: number, flags: number): number;

		/** Get file size */
		xFileSize(fileId: number, pSize64: DataView): number;

		/** Lock a file */
		xLock(fileId: number, flags: number): number;

		/** Unlock a file */
		xUnlock(fileId: number, flags: number): number;

		/** Check for reserved lock */
		xCheckReservedLock(fileId: number, pResOut: DataView): number;

		/** File control operations */
		xFileControl(fileId: number, op: number, pArg: DataView): number;

		/** Get sector size */
		xSectorSize(fileId: number): number;

		/** Get device characteristics */
		xDeviceCharacteristics(fileId: number): number;

		/** Open a file */
		xOpen(
			name: string | null,
			fileId: number,
			flags: number,
			pOutFlags: DataView,
		): number;

		/** Delete a file */
		xDelete(name: string, syncDir: number): number;

		/** Check file accessibility */
		xAccess(name: string, flags: number, pResOut: DataView): number;

		/** Handle asynchronous operations */
		handleAsync<T>(fn: () => Promise<T>): T;
	}
}

declare module "wa-sqlite/dist/wa-sqlite-async.mjs" {
	const factory: (options?: { wasmBinary?: ArrayBuffer | Uint8Array }) => Promise<any>;
	export default factory;
}

declare module "wa-sqlite" {
	export function Factory(module: any): any;
}

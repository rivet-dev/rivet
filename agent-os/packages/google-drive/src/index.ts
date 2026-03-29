/**
 * Google Drive-backed FsBlockStore.
 *
 * Stores blocks as files in a Google Drive folder using the Drive API v3.
 * Block key "ino/chunkIndex" maps to a file named "{keyPrefix}{key}" in the
 * configured folder.
 *
 * Implements the FsBlockStore interface from @secure-exec/core so it can be
 * composed with any FsMetadataStore via ChunkedVFS.
 *
 * **Preview**: This package is in preview and may have breaking changes.
 */

import { google } from "googleapis";
import type { drive_v3 } from "googleapis";
import { KernelError } from "@secure-exec/core";
import type { FsBlockStore } from "@secure-exec/core";

export interface GoogleDriveCredentials {
	/** Google service account client email. */
	clientEmail: string;
	/** Google service account private key (PEM format). */
	privateKey: string;
}

export interface GoogleDriveBlockStoreOptions {
	/** Google service account credentials. */
	credentials: GoogleDriveCredentials;
	/** Google Drive folder ID where blocks are stored. */
	folderId: string;
	/** Optional prefix for block file names. */
	keyPrefix?: string;
}

function normalizePrefix(raw: string | undefined): string {
	if (!raw || raw === "") return "";
	return raw.endsWith("/") ? raw : `${raw}/`;
}

export class GoogleDriveBlockStore implements FsBlockStore {
	private drive: drive_v3.Drive;
	private folderId: string;
	private prefix: string;
	/** Cache file name -> Drive file ID to avoid repeated lookups. */
	private fileIdCache: Map<string, string> = new Map();

	constructor(options: GoogleDriveBlockStoreOptions) {
		this.folderId = options.folderId;
		this.prefix = normalizePrefix(options.keyPrefix);

		const auth = new google.auth.JWT({
			email: options.credentials.clientEmail,
			key: options.credentials.privateKey,
			scopes: ["https://www.googleapis.com/auth/drive.file"],
		});
		this.drive = google.drive({ version: "v3", auth });
	}

	private fileName(key: string): string {
		return `${this.prefix}${key}`;
	}

	/**
	 * Find the Drive file ID for a given block key.
	 * Returns null if the file does not exist.
	 */
	private async findFileId(key: string): Promise<string | null> {
		const name = this.fileName(key);
		const cached = this.fileIdCache.get(name);
		if (cached) return cached;

		const escapedName = name.replace(/'/g, "\\'");
		const res = await this.drive.files.list({
			q: `name = '${escapedName}' and '${this.folderId}' in parents and trashed = false`,
			fields: "files(id)",
			pageSize: 1,
		});

		const fileId = res.data.files?.[0]?.id ?? null;
		if (fileId) {
			this.fileIdCache.set(name, fileId);
		}
		return fileId;
	}

	async read(key: string): Promise<Uint8Array> {
		const fileId = await this.findFileId(key);
		if (!fileId) {
			throw new KernelError("ENOENT", `block not found: ${key}`);
		}

		const res = await this.drive.files.get(
			{ fileId, alt: "media" },
			{ responseType: "arraybuffer" },
		);
		return new Uint8Array(res.data as ArrayBuffer);
	}

	async readRange(
		key: string,
		offset: number,
		length: number,
	): Promise<Uint8Array> {
		const fileId = await this.findFileId(key);
		if (!fileId) {
			throw new KernelError("ENOENT", `block not found: ${key}`);
		}

		try {
			const res = await this.drive.files.get(
				{ fileId, alt: "media" },
				{
					responseType: "arraybuffer",
					headers: {
						Range: `bytes=${offset}-${offset + length - 1}`,
					},
				},
			);
			return new Uint8Array(res.data as ArrayBuffer);
		} catch (err) {
			// Range not satisfiable means offset is beyond file size.
			if (isRangeError(err)) {
				return new Uint8Array(0);
			}
			throw err;
		}
	}

	async write(key: string, data: Uint8Array): Promise<void> {
		const name = this.fileName(key);
		const existingId = await this.findFileId(key);

		if (existingId) {
			// Update existing file.
			await this.drive.files.update({
				fileId: existingId,
				media: {
					mimeType: "application/octet-stream",
					body: bufferFromUint8Array(data),
				},
			});
		} else {
			// Create new file.
			const res = await this.drive.files.create({
				requestBody: {
					name,
					parents: [this.folderId],
					mimeType: "application/octet-stream",
				},
				media: {
					mimeType: "application/octet-stream",
					body: bufferFromUint8Array(data),
				},
				fields: "id",
			});
			const newId = res.data.id;
			if (newId) {
				this.fileIdCache.set(name, newId);
			}
		}
	}

	async delete(key: string): Promise<void> {
		const fileId = await this.findFileId(key);
		if (!fileId) return; // No-op for nonexistent keys.

		try {
			await this.drive.files.delete({ fileId });
		} catch (err) {
			// Ignore 404 (already deleted / race condition).
			if (!isNotFound(err)) throw err;
		}
		this.fileIdCache.delete(this.fileName(key));
	}

	async deleteMany(keys: string[]): Promise<void> {
		if (keys.length === 0) return;

		// Google Drive does not have a batch delete API like S3.
		// Delete sequentially to respect rate limits.
		const errors: Array<{ key: string; error: unknown }> = [];
		for (const key of keys) {
			try {
				await this.delete(key);
			} catch (err) {
				errors.push({ key, error: err });
			}
		}

		if (errors.length > 0) {
			const failedKeys = errors.map((e) => e.key).join(", ");
			throw new Error(
				`Failed to delete ${errors.length} block(s): ${failedKeys}`,
			);
		}
	}

	async copy(srcKey: string, dstKey: string): Promise<void> {
		const srcFileId = await this.findFileId(srcKey);
		if (!srcFileId) {
			throw new KernelError("ENOENT", `block not found: ${srcKey}`);
		}

		const dstName = this.fileName(dstKey);

		// Remove existing destination if present.
		const existingDstId = await this.findFileId(dstKey);
		if (existingDstId) {
			try {
				await this.drive.files.delete({ fileId: existingDstId });
			} catch (err) {
				if (!isNotFound(err)) throw err;
			}
			this.fileIdCache.delete(dstName);
		}

		// Server-side copy.
		const res = await this.drive.files.copy({
			fileId: srcFileId,
			requestBody: {
				name: dstName,
				parents: [this.folderId],
			},
			fields: "id",
		});

		const newId = res.data.id;
		if (newId) {
			this.fileIdCache.set(dstName, newId);
		}
	}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function bufferFromUint8Array(data: Uint8Array): Buffer {
	return Buffer.from(data.buffer, data.byteOffset, data.byteLength);
}

function isNotFound(err: unknown): boolean {
	if (typeof err !== "object" || err === null) return false;
	const e = err as { code?: number; status?: number };
	return e.code === 404 || e.status === 404;
}

function isRangeError(err: unknown): boolean {
	if (typeof err !== "object" || err === null) return false;
	const e = err as { code?: number; status?: number };
	return e.code === 416 || e.status === 416;
}

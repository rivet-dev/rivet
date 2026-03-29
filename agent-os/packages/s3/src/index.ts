/**
 * S3-backed FsBlockStore.
 *
 * Stores blocks as objects in S3-compatible storage (AWS S3, MinIO, etc.).
 * Block key "ino/chunkIndex" maps to S3 object key "{prefix}blocks/{key}".
 *
 * Implements the FsBlockStore interface from @secure-exec/core so it can be
 * composed with any FsMetadataStore via ChunkedVFS.
 */

import {
	CopyObjectCommand,
	DeleteObjectCommand,
	DeleteObjectsCommand,
	GetObjectCommand,
	PutObjectCommand,
	S3Client,
} from "@aws-sdk/client-s3";
import {
	KernelError,
	createChunkedVfs,
	InMemoryMetadataStore,
} from "@secure-exec/core";
import type { FsBlockStore, VirtualFileSystem } from "@secure-exec/core";
import type { AgentOsContext } from "@rivet-dev/agent-os-core";
export type { AgentOsContext } from "@rivet-dev/agent-os-core";

export interface S3BlockStoreOptions {
	/** S3 bucket name. */
	bucket: string;
	/** Key prefix prepended to all block keys (e.g. "vm-1/"). Trailing slash added automatically. */
	prefix?: string;
	/** AWS region (default "us-east-1"). */
	region?: string;
	/** Explicit credentials (otherwise uses default SDK chain). */
	credentials?: { accessKeyId: string; secretAccessKey: string };
	/** Custom S3-compatible endpoint URL (e.g. for MinIO). */
	endpoint?: string;
}

function normalizePrefix(raw: string | undefined): string {
	if (!raw || raw === "") return "";
	return raw.endsWith("/") ? raw : `${raw}/`;
}

export class S3BlockStore implements FsBlockStore {
	private client: S3Client;
	private bucket: string;
	private prefix: string;

	constructor(options: S3BlockStoreOptions) {
		this.bucket = options.bucket;
		this.prefix = normalizePrefix(options.prefix);
		this.client = new S3Client({
			region: options.region ?? "us-east-1",
			credentials: options.credentials,
			endpoint: options.endpoint,
			forcePathStyle: true,
		});
	}

	private objectKey(key: string): string {
		return `${this.prefix}blocks/${key}`;
	}

	async read(key: string): Promise<Uint8Array> {
		try {
			const resp = await this.client.send(
				new GetObjectCommand({
					Bucket: this.bucket,
					Key: this.objectKey(key),
				}),
			);
			const bytes = await resp.Body?.transformToByteArray();
			if (!bytes) {
				throw new KernelError("EIO", `empty response body: ${key}`);
			}
			return new Uint8Array(bytes);
		} catch (err) {
			if (err instanceof KernelError) throw err;
			if (isNoSuchKey(err)) {
				throw new KernelError("ENOENT", `block not found: ${key}`);
			}
			throw err;
		}
	}

	async readRange(
		key: string,
		offset: number,
		length: number,
	): Promise<Uint8Array> {
		try {
			const resp = await this.client.send(
				new GetObjectCommand({
					Bucket: this.bucket,
					Key: this.objectKey(key),
					Range: `bytes=${offset}-${offset + length - 1}`,
				}),
			);
			const bytes = await resp.Body?.transformToByteArray();
			if (!bytes) {
				return new Uint8Array(0);
			}
			return new Uint8Array(bytes);
		} catch (err) {
			if (err instanceof KernelError) throw err;
			if (isNoSuchKey(err)) {
				throw new KernelError("ENOENT", `block not found: ${key}`);
			}
			// InvalidRange means offset is beyond file size. Return empty for short read.
			const e = err as { name?: string; $metadata?: { httpStatusCode?: number } };
			if (e.name === "InvalidRange" || e.$metadata?.httpStatusCode === 416) {
				return new Uint8Array(0);
			}
			throw err;
		}
	}

	async write(key: string, data: Uint8Array): Promise<void> {
		await this.client.send(
			new PutObjectCommand({
				Bucket: this.bucket,
				Key: this.objectKey(key),
				Body: data,
			}),
		);
	}

	async delete(key: string): Promise<void> {
		// S3 DeleteObject is a no-op for nonexistent keys.
		await this.client.send(
			new DeleteObjectCommand({
				Bucket: this.bucket,
				Key: this.objectKey(key),
			}),
		);
	}

	async deleteMany(keys: string[]): Promise<void> {
		if (keys.length === 0) return;

		// S3 DeleteObjects supports up to 1000 keys per request.
		const batchSize = 1000;
		const failedKeys: string[] = [];
		for (let i = 0; i < keys.length; i += batchSize) {
			const batch = keys.slice(i, i + batchSize);
			try {
				const resp = await this.client.send(
					new DeleteObjectsCommand({
						Bucket: this.bucket,
						Delete: {
							Objects: batch.map((k) => ({ Key: this.objectKey(k) })),
							Quiet: true,
						},
					}),
				);
				if (resp.Errors && resp.Errors.length > 0) {
					for (const e of resp.Errors) {
						failedKeys.push(e.Key ?? "unknown");
					}
				}
			} catch {
				failedKeys.push(...batch);
			}
		}
		if (failedKeys.length > 0) {
			throw new Error(
				`S3 deleteMany failed for ${failedKeys.length} keys: ${failedKeys.slice(0, 10).join(", ")}${failedKeys.length > 10 ? "..." : ""}`,
			);
		}
	}

	async copy(srcKey: string, dstKey: string): Promise<void> {
		const srcObjectKey = this.objectKey(srcKey);
		const encodedSource = encodeURIComponent(
			`${this.bucket}/${srcObjectKey}`,
		).replace(/%2F/g, "/");
		try {
			await this.client.send(
				new CopyObjectCommand({
					Bucket: this.bucket,
					CopySource: encodedSource,
					Key: this.objectKey(dstKey),
				}),
			);
		} catch (err) {
			if (isNoSuchKey(err)) {
				throw new KernelError("ENOENT", `block not found: ${srcKey}`);
			}
			throw err;
		}
	}
}

function isNoSuchKey(err: unknown): boolean {
	if (typeof err !== "object" || err === null) return false;
	const e = err as { name?: string };
	return e.name === "NoSuchKey" || e.name === "NotFound";
}

// ---------------------------------------------------------------------------
// Backward-compatible wrapper (removed in US-016)
// ---------------------------------------------------------------------------

/** @deprecated Use S3BlockStore with ChunkedVFS instead. */
export interface S3FsOptions {
	bucket: string;
	prefix?: string;
	region?: string;
	credentials?: { accessKeyId: string; secretAccessKey: string };
	endpoint?: string;
}

/**
 * Create a VirtualFileSystem backed by S3 via ChunkedVFS.
 * @deprecated Use S3BlockStore with ChunkedVFS directly.
 */
export function createS3Backend(options: S3FsOptions): VirtualFileSystem {
	return createChunkedVfs({
		metadata: new InMemoryMetadataStore(),
		blocks: new S3BlockStore(options),
	});
}

// ---------------------------------------------------------------------------
// Context-aware factory
// ---------------------------------------------------------------------------

/** Options for createS3BackendForAgent. Same as S3BlockStoreOptions but prefix is derived from context. */
export interface S3AgentOptions {
	/** S3 bucket name. */
	bucket: string;
	/** AWS region (default "us-east-1"). */
	region?: string;
	/** Explicit credentials (otherwise uses default SDK chain). */
	credentials?: { accessKeyId: string; secretAccessKey: string };
	/** Custom S3-compatible endpoint URL (e.g. for MinIO). */
	endpoint?: string;
}

/**
 * Create an S3-backed VirtualFileSystem with a prefix derived from the agent context.
 *
 * The S3 key prefix is set to `agents/{context.key}/` so each agent instance
 * gets its own isolated storage namespace within the bucket.
 */
export function createS3BackendForAgent(
	context: AgentOsContext,
	options: S3AgentOptions,
): VirtualFileSystem {
	return createChunkedVfs({
		metadata: new InMemoryMetadataStore(),
		blocks: new S3BlockStore({
			...options,
			prefix: `agents/${context.key}/`,
		}),
	});
}

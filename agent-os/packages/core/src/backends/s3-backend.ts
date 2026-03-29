/**
 * S3 mount backend.
 *
 * Stores files in S3-compatible object storage. Supports any
 * S3-compatible endpoint (AWS, MinIO, etc.) via the endpoint option.
 */

import {
	CopyObjectCommand,
	DeleteObjectCommand,
	GetObjectCommand,
	HeadObjectCommand,
	ListObjectsV2Command,
	PutObjectCommand,
	S3Client,
} from "@aws-sdk/client-s3";
import {
	KernelError,
	type VirtualDirEntry,
	type VirtualFileSystem,
	type VirtualStat,
} from "@secure-exec/core";

export interface S3BackendOptions {
	/** S3 bucket name. */
	bucket: string;
	/** Key prefix prepended to all paths (e.g. "vm-1/"). */
	prefix?: string;
	/** AWS region. */
	region?: string;
	/** Explicit credentials (otherwise uses default SDK chain). */
	credentials?: { accessKeyId: string; secretAccessKey: string };
	/** Custom S3-compatible endpoint URL (e.g. for MinIO or mock servers). */
	endpoint?: string;
}

function makeKey(prefix: string, p: string): string {
	const normalized = p.replace(/^\/+/, "");
	return prefix + normalized;
}

function makeDirPrefix(prefix: string, p: string): string {
	const key = makeKey(prefix, p);
	return key === "" || key.endsWith("/") ? key : `${key}/`;
}

/**
 * Create a VirtualFileSystem backed by S3-compatible object storage.
 */
export function createS3Backend(options: S3BackendOptions): VirtualFileSystem {
	const prefix = options.prefix ?? "";
	const bucket = options.bucket;

	const client = new S3Client({
		region: options.region ?? "us-east-1",
		credentials: options.credentials,
		endpoint: options.endpoint,
		forcePathStyle: !!options.endpoint,
	});

	function makeStat(
		size: number,
		isDir: boolean,
		lastModified?: Date,
	): VirtualStat {
		const now = Date.now();
		const mtime = lastModified ? lastModified.getTime() : now;
		return {
			mode: isDir ? 0o40755 : 0o100644,
			size,
			isDirectory: isDir,
			isSymbolicLink: false,
			atimeMs: mtime,
			mtimeMs: mtime,
			ctimeMs: mtime,
			birthtimeMs: mtime,
			ino: 0,
			nlink: 1,
			uid: 0,
			gid: 0,
		};
	}

	const backend: VirtualFileSystem = {
		async readFile(p: string): Promise<Uint8Array> {
			try {
				const resp = await client.send(
					new GetObjectCommand({
						Bucket: bucket,
						Key: makeKey(prefix, p),
					}),
				);
				const bytes = await resp.Body?.transformToByteArray();
				if (!bytes)
					throw new KernelError("EIO", `empty response body: ${p}`);
				return new Uint8Array(bytes);
			} catch (err) {
				if (err instanceof KernelError) throw err;
				const e = err as { name?: string };
				if (e.name === "NoSuchKey" || e.name === "NotFound") {
					throw new KernelError("ENOENT", `no such file: ${p}`);
				}
				throw err;
			}
		},

		async readTextFile(p: string): Promise<string> {
			const data = await backend.readFile(p);
			return new TextDecoder().decode(data);
		},

		async readDir(p: string): Promise<string[]> {
			const dirPrefix = makeDirPrefix(prefix, p);
			const resp = await client.send(
				new ListObjectsV2Command({
					Bucket: bucket,
					Prefix: dirPrefix,
					Delimiter: "/",
				}),
			);

			const names: string[] = [];
			// Files (Contents)
			if (resp.Contents) {
				for (const obj of resp.Contents) {
					if (!obj.Key || obj.Key === dirPrefix) continue;
					const name = obj.Key.slice(dirPrefix.length);
					if (name && !name.includes("/")) {
						names.push(name);
					}
				}
			}
			// Subdirectories (CommonPrefixes)
			if (resp.CommonPrefixes) {
				for (const cp of resp.CommonPrefixes) {
					if (!cp.Prefix) continue;
					const name = cp.Prefix.slice(dirPrefix.length).replace(
						/\/$/,
						"",
					);
					if (name) {
						names.push(name);
					}
				}
			}
			return names;
		},

		async readDirWithTypes(p: string): Promise<VirtualDirEntry[]> {
			const dirPrefix = makeDirPrefix(prefix, p);
			const resp = await client.send(
				new ListObjectsV2Command({
					Bucket: bucket,
					Prefix: dirPrefix,
					Delimiter: "/",
				}),
			);

			const entries: VirtualDirEntry[] = [];
			if (resp.Contents) {
				for (const obj of resp.Contents) {
					if (!obj.Key || obj.Key === dirPrefix) continue;
					const name = obj.Key.slice(dirPrefix.length);
					if (name && !name.includes("/")) {
						entries.push({
							name,
							isDirectory: false,
							isSymbolicLink: false,
						});
					}
				}
			}
			if (resp.CommonPrefixes) {
				for (const cp of resp.CommonPrefixes) {
					if (!cp.Prefix) continue;
					const name = cp.Prefix.slice(dirPrefix.length).replace(
						/\/$/,
						"",
					);
					if (name) {
						entries.push({
							name,
							isDirectory: true,
							isSymbolicLink: false,
						});
					}
				}
			}
			return entries;
		},

		async writeFile(
			p: string,
			content: string | Uint8Array,
		): Promise<void> {
			const body =
				typeof content === "string"
					? new TextEncoder().encode(content)
					: content;
			await client.send(
				new PutObjectCommand({
					Bucket: bucket,
					Key: makeKey(prefix, p),
					Body: body,
				}),
			);
		},

		async createDir(_p: string): Promise<void> {
			// S3 directories are implicit; no-op
		},

		async mkdir(
			_p: string,
			_options?: { recursive?: boolean },
		): Promise<void> {
			// S3 directories are implicit; no-op
		},

		async exists(p: string): Promise<boolean> {
			try {
				await client.send(
					new HeadObjectCommand({
						Bucket: bucket,
						Key: makeKey(prefix, p),
					}),
				);
				return true;
			} catch (err) {
				const e = err as { name?: string };
				if (e.name === "NotFound" || e.name === "NoSuchKey") {
					// Also check if it's a "directory" (has objects with this prefix)
					const dirPrefix = makeDirPrefix(prefix, p);
					const resp = await client.send(
						new ListObjectsV2Command({
							Bucket: bucket,
							Prefix: dirPrefix,
							MaxKeys: 1,
						}),
					);
					return (resp.Contents?.length ?? 0) > 0;
				}
				throw err;
			}
		},

		async stat(p: string): Promise<VirtualStat> {
			try {
				const resp = await client.send(
					new HeadObjectCommand({
						Bucket: bucket,
						Key: makeKey(prefix, p),
					}),
				);
				return makeStat(
					resp.ContentLength ?? 0,
					false,
					resp.LastModified,
				);
			} catch (err) {
				const e = err as { name?: string };
				if (e.name === "NotFound" || e.name === "NoSuchKey") {
					// Check if it's a "directory"
					const dirPrefix = makeDirPrefix(prefix, p);
					const resp = await client.send(
						new ListObjectsV2Command({
							Bucket: bucket,
							Prefix: dirPrefix,
							MaxKeys: 1,
						}),
					);
					if ((resp.Contents?.length ?? 0) > 0) {
						return makeStat(0, true);
					}
					throw new KernelError(
						"ENOENT",
						`no such file or directory: ${p}`,
					);
				}
				throw err;
			}
		},

		async removeFile(p: string): Promise<void> {
			await client.send(
				new DeleteObjectCommand({
					Bucket: bucket,
					Key: makeKey(prefix, p),
				}),
			);
		},

		async removeDir(_p: string): Promise<void> {
			// S3 directories are implicit; no-op
		},

		async rename(oldPath: string, newPath: string): Promise<void> {
			const sourceKey = makeKey(prefix, oldPath);
			const destKey = makeKey(prefix, newPath);
			await client.send(
				new CopyObjectCommand({
					Bucket: bucket,
					CopySource: `${bucket}/${sourceKey}`,
					Key: destKey,
				}),
			);
			await client.send(
				new DeleteObjectCommand({ Bucket: bucket, Key: sourceKey }),
			);
		},

		async realpath(p: string): Promise<string> {
			return p;
		},

		async symlink(_target: string, _linkPath: string): Promise<void> {
			throw new KernelError(
				"ENOSYS",
				"symlink not supported by S3 backend",
			);
		},

		async readlink(_p: string): Promise<string> {
			throw new KernelError(
				"ENOSYS",
				"readlink not supported by S3 backend",
			);
		},

		async lstat(p: string): Promise<VirtualStat> {
			return backend.stat(p);
		},

		async link(_oldPath: string, _newPath: string): Promise<void> {
			throw new KernelError("ENOSYS", "link not supported by S3 backend");
		},

		async chmod(_p: string, _mode: number): Promise<void> {
			throw new KernelError(
				"ENOSYS",
				"chmod not supported by S3 backend",
			);
		},

		async chown(_p: string, _uid: number, _gid: number): Promise<void> {
			throw new KernelError(
				"ENOSYS",
				"chown not supported by S3 backend",
			);
		},

		async utimes(
			_p: string,
			_atime: number,
			_mtime: number,
		): Promise<void> {
			// S3 doesn't support setting arbitrary timestamps; no-op
		},

		async truncate(p: string, length: number): Promise<void> {
			if (length === 0) {
				await backend.writeFile(p, new Uint8Array(0));
				return;
			}
			const data = await backend.readFile(p);
			await backend.writeFile(p, data.slice(0, length));
		},

		async pread(
			p: string,
			offset: number,
			length: number,
		): Promise<Uint8Array> {
			try {
				const resp = await client.send(
					new GetObjectCommand({
						Bucket: bucket,
						Key: makeKey(prefix, p),
						Range: `bytes=${offset}-${offset + length - 1}`,
					}),
				);
				const bytes = await resp.Body?.transformToByteArray();
				if (!bytes)
					throw new KernelError("EIO", `empty response body: ${p}`);
				return new Uint8Array(bytes);
			} catch (err) {
				if (err instanceof KernelError) throw err;
				const e = err as { name?: string };
				if (e.name === "NoSuchKey" || e.name === "NotFound") {
					throw new KernelError("ENOENT", `no such file: ${p}`);
				}
				throw err;
			}
		},
	};

	return backend;
}

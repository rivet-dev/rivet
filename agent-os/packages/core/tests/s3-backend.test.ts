import * as http from "node:http";
import { afterAll, beforeAll, describe, expect, test } from "vitest";
import { createS3Backend } from "../src/backends/s3-backend.js";

/**
 * Minimal mock S3 server that stores objects in memory.
 * Supports: PUT, GET, HEAD, DELETE, GET with list-type=2 (ListObjectsV2).
 */
function createMockS3Server() {
	const store = new Map<string, { body: Buffer; lastModified: Date }>();

	const server = http.createServer((req, res) => {
		const url = new URL(req.url as string, `http://${req.headers.host}`);
		// Path format: /bucket/key...
		const pathParts = url.pathname.split("/").filter(Boolean);
		const bucket = pathParts[0];
		const key = pathParts.slice(1).join("/");
		const fullKey = `${bucket}/${key}`;

		// ListObjectsV2
		if (req.method === "GET" && url.searchParams.get("list-type") === "2") {
			const prefix = url.searchParams.get("prefix") ?? "";
			const delimiter = url.searchParams.get("delimiter") ?? "";
			const bucketPrefix = `${bucket}/`;

			const contents: string[] = [];
			const commonPrefixes = new Set<string>();

			for (const [k] of store) {
				if (!k.startsWith(bucketPrefix)) continue;
				const objKey = k.slice(bucketPrefix.length);
				if (!objKey.startsWith(prefix)) continue;

				const rest = objKey.slice(prefix.length);
				if (delimiter && rest.includes(delimiter)) {
					const cpEnd = rest.indexOf(delimiter) + delimiter.length;
					commonPrefixes.add(prefix + rest.slice(0, cpEnd));
				} else {
					contents.push(objKey);
				}
			}

			let xml = '<?xml version="1.0" encoding="UTF-8"?>';
			xml += "<ListBucketResult>";
			xml += `<Name>${bucket}</Name>`;
			xml += `<Prefix>${prefix}</Prefix>`;
			xml += `<Delimiter>${delimiter}</Delimiter>`;
			xml += `<KeyCount>${contents.length + commonPrefixes.size}</KeyCount>`;
			for (const objKey of contents) {
				const entry = store.get(`${bucketPrefix}${objKey}`);
				xml += "<Contents>";
				xml += `<Key>${objKey}</Key>`;
				xml += `<Size>${entry?.body.length ?? 0}</Size>`;
				xml += `<LastModified>${(entry?.lastModified ?? new Date()).toISOString()}</LastModified>`;
				xml += "</Contents>";
			}
			for (const cp of commonPrefixes) {
				xml += `<CommonPrefixes><Prefix>${cp}</Prefix></CommonPrefixes>`;
			}
			xml += "</ListBucketResult>";

			res.writeHead(200, { "Content-Type": "application/xml" });
			res.end(xml);
			return;
		}

		// CopyObject (PUT with x-amz-copy-source)
		const copySource = req.headers["x-amz-copy-source"] as
			| string
			| undefined;
		if (req.method === "PUT" && copySource) {
			const sourceKey = decodeURIComponent(copySource.replace(/^\//, ""));
			const entry = store.get(sourceKey);
			if (!entry) {
				res.writeHead(404);
				res.end(
					'<?xml version="1.0"?><Error><Code>NoSuchKey</Code></Error>',
				);
				return;
			}
			store.set(fullKey, {
				body: Buffer.from(entry.body),
				lastModified: new Date(),
			});
			res.writeHead(200, { "Content-Type": "application/xml" });
			res.end(
				'<?xml version="1.0"?><CopyObjectResult><ETag>"copy"</ETag></CopyObjectResult>',
			);
			return;
		}

		if (req.method === "PUT") {
			const chunks: Buffer[] = [];
			req.on("data", (chunk) => chunks.push(chunk));
			req.on("end", () => {
				store.set(fullKey, {
					body: Buffer.concat(chunks),
					lastModified: new Date(),
				});
				res.writeHead(200);
				res.end();
			});
			return;
		}

		if (req.method === "GET") {
			const entry = store.get(fullKey);
			if (!entry) {
				res.writeHead(404);
				res.end(
					'<?xml version="1.0"?><Error><Code>NoSuchKey</Code></Error>',
				);
				return;
			}
			const rangeHeader = req.headers.range;
			if (rangeHeader) {
				const match = rangeHeader.match(/bytes=(\d+)-(\d+)/);
				if (match) {
					const start = Number.parseInt(match[1], 10);
					const end = Math.min(
						Number.parseInt(match[2], 10),
						entry.body.length - 1,
					);
					const slice = entry.body.slice(start, end + 1);
					res.writeHead(206, {
						"Content-Length": String(slice.length),
						"Content-Range": `bytes ${start}-${end}/${entry.body.length}`,
						"Last-Modified": entry.lastModified.toUTCString(),
					});
					res.end(slice);
					return;
				}
			}
			res.writeHead(200, {
				"Content-Length": String(entry.body.length),
				"Last-Modified": entry.lastModified.toUTCString(),
			});
			res.end(entry.body);
			return;
		}

		if (req.method === "HEAD") {
			const entry = store.get(fullKey);
			if (!entry) {
				res.writeHead(404);
				res.end();
				return;
			}
			res.writeHead(200, {
				"Content-Length": String(entry.body.length),
				"Last-Modified": entry.lastModified.toUTCString(),
			});
			res.end();
			return;
		}

		if (req.method === "DELETE") {
			store.delete(fullKey);
			res.writeHead(204);
			res.end();
			return;
		}

		res.writeHead(405);
		res.end();
	});

	return { server, store };
}

describe("S3Backend", () => {
	let server: http.Server;
	let store: Map<string, { body: Buffer; lastModified: Date }>;
	let endpoint: string;

	beforeAll(async () => {
		const mock = createMockS3Server();
		server = mock.server;
		store = mock.store;

		await new Promise<void>((resolve) => {
			server.listen(0, "127.0.0.1", () => resolve());
		});
		const addr = server.address() as { port: number };
		endpoint = `http://127.0.0.1:${addr.port}`;
	});

	afterAll(async () => {
		await new Promise<void>((resolve) => {
			server.close(() => resolve());
		});
	});

	function makeBackend(prefix?: string) {
		return createS3Backend({
			bucket: "test-bucket",
			prefix,
			region: "us-east-1",
			credentials: {
				accessKeyId: "test",
				secretAccessKey: "test",
			},
			endpoint,
		});
	}

	test("writeFile and readFile round-trip", async () => {
		const s3 = makeBackend();
		await s3.writeFile("hello.txt", "hello world");
		const data = await s3.readFile("hello.txt");
		expect(new TextDecoder().decode(data)).toBe("hello world");
	});

	test("readTextFile returns string", async () => {
		const s3 = makeBackend();
		await s3.writeFile("text.txt", "some text");
		const text = await s3.readTextFile("text.txt");
		expect(text).toBe("some text");
	});

	test("readFile throws ENOENT for missing file", async () => {
		const s3 = makeBackend("miss/");
		await expect(s3.readFile("nonexistent.txt")).rejects.toThrow("ENOENT");
	});

	test("writeFile with Uint8Array", async () => {
		const s3 = makeBackend();
		const bytes = new Uint8Array([1, 2, 3, 4, 5]);
		await s3.writeFile("binary.bin", bytes);
		const data = await s3.readFile("binary.bin");
		expect(data).toEqual(bytes);
	});

	test("exists returns true for existing file", async () => {
		const s3 = makeBackend("exists/");
		await s3.writeFile("file.txt", "data");
		expect(await s3.exists("file.txt")).toBe(true);
	});

	test("exists returns false for missing file", async () => {
		const s3 = makeBackend("noexist/");
		expect(await s3.exists("nope.txt")).toBe(false);
	});

	test("exists returns true for directory prefix", async () => {
		const s3 = makeBackend("dircheck/");
		await s3.writeFile("subdir/file.txt", "data");
		expect(await s3.exists("subdir")).toBe(true);
	});

	test("stat returns size and mtime from HeadObject", async () => {
		const s3 = makeBackend("stat/");
		await s3.writeFile("sized.txt", "12345");
		const st = await s3.stat("sized.txt");
		expect(st.size).toBe(5);
		expect(st.isDirectory).toBe(false);
		expect(st.mtimeMs).toBeGreaterThan(0);
	});

	test("stat returns directory for prefix", async () => {
		const s3 = makeBackend("statdir/");
		await s3.writeFile("sub/file.txt", "data");
		const st = await s3.stat("sub");
		expect(st.isDirectory).toBe(true);
	});

	test("stat throws ENOENT for missing path", async () => {
		const s3 = makeBackend("statnone/");
		await expect(s3.stat("ghost.txt")).rejects.toThrow("ENOENT");
	});

	test("removeFile deletes object", async () => {
		const s3 = makeBackend("del/");
		await s3.writeFile("gone.txt", "bye");
		await s3.removeFile("gone.txt");
		expect(await s3.exists("gone.txt")).toBe(false);
	});

	test("readdir lists files and subdirectories", async () => {
		// Clear previous state by using unique prefix
		const s3 = makeBackend("readdir/");
		await s3.writeFile("a.txt", "a");
		await s3.writeFile("b.txt", "b");
		await s3.writeFile("sub/c.txt", "c");

		const entries = await s3.readDir("/");
		expect(entries).toContain("a.txt");
		expect(entries).toContain("b.txt");
		expect(entries).toContain("sub");
		expect(entries).not.toContain("c.txt");
	});

	test("readdir with prefix returns correct nested files", async () => {
		const s3 = makeBackend("readdirpfx/");
		await s3.writeFile("dir/x.txt", "x");
		await s3.writeFile("dir/y.txt", "y");
		await s3.writeFile("dir/nested/z.txt", "z");

		const entries = await s3.readDir("dir");
		expect(entries).toContain("x.txt");
		expect(entries).toContain("y.txt");
		expect(entries).toContain("nested");
	});

	test("readDirWithTypes returns typed entries", async () => {
		const s3 = makeBackend("rdwt/");
		await s3.writeFile("file.txt", "data");
		await s3.writeFile("folder/inner.txt", "data");

		const entries = await s3.readDirWithTypes("/");
		const file = entries.find((e) => e.name === "file.txt");
		const dir = entries.find((e) => e.name === "folder");
		expect(file?.isDirectory).toBe(false);
		expect(dir?.isDirectory).toBe(true);
	});

	test("rename copies then deletes source", async () => {
		const s3 = makeBackend("rename/");
		await s3.writeFile("old.txt", "content");
		await s3.rename("old.txt", "new.txt");

		const data = await s3.readTextFile("new.txt");
		expect(data).toBe("content");
		expect(await s3.exists("old.txt")).toBe(false);
	});

	test("prefix is prepended to all keys", async () => {
		const s3 = makeBackend("pfx/");
		await s3.writeFile("key.txt", "val");

		// Verify in raw store
		expect(store.has("test-bucket/pfx/key.txt")).toBe(true);
	});

	test("symlink throws ENOSYS", async () => {
		const s3 = makeBackend();
		await expect(s3.symlink("a", "b")).rejects.toThrow("ENOSYS");
	});

	test("link throws ENOSYS", async () => {
		const s3 = makeBackend();
		await expect(s3.link("a", "b")).rejects.toThrow("ENOSYS");
	});

	test("chmod throws ENOSYS", async () => {
		const s3 = makeBackend();
		await expect(s3.chmod("a", 0o644)).rejects.toThrow("ENOSYS");
	});

	test("chown throws ENOSYS", async () => {
		const s3 = makeBackend();
		await expect(s3.chown("a", 0, 0)).rejects.toThrow("ENOSYS");
	});

	test("mkdir is a no-op (does not throw)", async () => {
		const s3 = makeBackend();
		await expect(s3.mkdir("some-dir")).resolves.toBeUndefined();
	});

	test("pread reads partial content", async () => {
		const s3 = makeBackend("pread/");
		await s3.writeFile("data.txt", "abcdefghij");
		const slice = await s3.pread("data.txt", 2, 4);
		expect(new TextDecoder().decode(slice)).toBe("cdef");
	});
});

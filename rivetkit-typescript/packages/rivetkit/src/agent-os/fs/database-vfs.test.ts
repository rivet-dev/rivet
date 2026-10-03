import { DatabaseSync } from "node:sqlite";
import { afterEach, describe, expect, test } from "vitest";
import type { RawAccess } from "@/common/database/config";
import { migrateAgentOsTables } from "../actor/db";
import { createDatabaseVfs } from "./database-vfs";

const databases: DatabaseSync[] = [];

afterEach(() => {
	for (const database of databases.splice(0)) database.close();
});

async function createVfs() {
	const raw = new DatabaseSync(":memory:");
	databases.push(raw);
	const db = {
		execute: async <TRow extends Record<string, unknown>>(
			sql: string,
			...args: unknown[]
		): Promise<TRow[]> => {
			if (args.length > 0 || /^\s*SELECT\b/i.test(sql)) {
				return raw.prepare(sql).all(...(args as never[])) as TRow[];
			}
			raw.exec(sql);
			return [];
		},
	} as unknown as RawAccess;
	await migrateAgentOsTables(db);
	return createDatabaseVfs({ db });
}

describe("database vfs directory prefixes", () => {
	test("readDir does not list entries of a directory that only matches as a LIKE pattern", async () => {
		const vfs = await createVfs();
		await vfs.mkdir("/my_dir");
		await vfs.mkdir("/myXdir");
		await vfs.mkdir("/MY_DIR");
		await vfs.writeFile("/myXdir/other.txt", "other");
		await vfs.writeFile("/MY_DIR/upper.txt", "upper");
		await vfs.writeFile("/my_dir/own.txt", "own");

		expect(await vfs.readDir("/my_dir")).toEqual(["own.txt"]);
	});

	test("removeDir removes an empty directory whose name has LIKE wildcards", async () => {
		const vfs = await createVfs();
		await vfs.mkdir("/a_b");
		await vfs.mkdir("/a%b");
		await vfs.mkdir("/aXb");
		await vfs.writeFile("/aXb/keep.txt", "keep");

		await vfs.removeDir("/a_b");
		await vfs.removeDir("/a%b");

		expect(await vfs.exists("/a_b")).toBe(false);
		expect(await vfs.exists("/a%b")).toBe(false);
	});

	test("rename of a directory only moves its own descendants", async () => {
		const vfs = await createVfs();
		await vfs.mkdir("/work_dir");
		await vfs.mkdir("/workXdir");
		await vfs.mkdir("/Work_Dir");
		await vfs.writeFile("/work_dir/mine.txt", "mine");
		await vfs.writeFile("/workXdir/sibling.txt", "sibling");
		await vfs.writeFile("/Work_Dir/upper.txt", "upper");

		await vfs.rename("/work_dir", "/moved");

		expect(await vfs.readDir("/moved")).toEqual(["mine.txt"]);
		expect(await vfs.readTextFile("/workXdir/sibling.txt")).toBe("sibling");
		expect(await vfs.readTextFile("/Work_Dir/upper.txt")).toBe("upper");
	});
});

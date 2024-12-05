import { Query, Database, ScriptContext } from "../module.gen.ts";

export interface Request extends Record<string, never> {}

export interface Response {
	id: string;
}

export type IdentityType = { guest: IdentityTypeGuest };

export interface IdentityTypeGuest {
}

export async function run(
	ctx: ScriptContext,
	_req: Request,
): Promise<Response> {
	// Create entry
	await ctx.db.insert(Database.dbEntry).values({ test2: "abc" });

	// Get entry
	const entry = await ctx.db.query.dbEntry.findFirst({
		where: Query.eq(Database.dbEntry.test2, "abc"),
	});

	return {
		id: entry!.id,
	};
}

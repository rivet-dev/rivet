// Used by the test in isolate.rs

export default {
	async start(ctx) {
		console.log(ctx);

		await ctx.kv.putBatch(new Map([[['foob', 'b'], 12], [['foob', 'a'], null], [['foob', 'c'], true]]));

		let res = await ctx.kv.list({ prefix: ['foob'] });

		console.log(res.array(), res.raw(), res.entries());
		console.log(res.get(['foob', 'b']));

		Deno.exit(2);

		throw new Error('bingus');
	}
};

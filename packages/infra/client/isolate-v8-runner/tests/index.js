// Used by the test in isolate.rs

export default {
	async start(ctx) {
		console.log(ctx);

		await ctx.kv.put(['foob', 'b'], 1);

		let res = await ctx.kv.getBatch(['foob', 'b']);
		console.log(res, res.get(['foob', 'b']));

		Deno.exit(2);

		throw new Error('bingus');
	}
};

import Rivet from "@rivet-gg/actors-core";

await Rivet.kv.put("count", 0);

console.log('Started', Deno.env.toObject());
setInterval(async () => {
  let x = await Rivet.kv.get("count");
  await Rivet.kv.put("count", x + 1);
  console.log('Count', x);
}, 1000);

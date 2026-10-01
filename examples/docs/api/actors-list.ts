const endpoint = "https://api.rivet.dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const token = process.env.RIVET_TOKEN ?? "";

const params = new URLSearchParams({ namespace, name: "counter" });
const response = await fetch(`${endpoint}/actors?${params}`, {
	headers: { Authorization: `Bearer ${token}` },
});

const { actors, pagination } = await response.json();
for (const actor of actors) {
	console.log(actor.actor_id, actor.name, actor.key);
}
// Pass pagination.cursor as ?cursor= to fetch the next page.
console.log(pagination.cursor);

export {};

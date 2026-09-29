const endpoint = "https://api.rivet.dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const token = process.env.RIVET_TOKEN ?? "";
const actorId = "00000000-0000-0000-0000-000000000000";

const response = await fetch(
	`${endpoint}/actors/${actorId}?namespace=${namespace}`,
	{
		method: "DELETE",
		headers: { Authorization: `Bearer ${token}` },
	},
);

if (!response.ok) {
	throw new Error(`Failed to destroy actor: ${await response.text()}`);
}

export {};

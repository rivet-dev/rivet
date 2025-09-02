export const RIVET_ENDPOINT =
	process.env.RIVET_ENDPOINT ?? "http://localhost:6420";

export async function createActor(
	namespaceName: string,
	runnerNameSelector: string,
): Promise<any> {
	const response = await fetch(
		`${RIVET_ENDPOINT}/actors?namespace=${namespaceName}`,
		{
			method: "POST",
			headers: {
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				name: "thingy",
				key: crypto.randomUUID(),
				input: btoa("hello"),
				runner_name_selector: runnerNameSelector,
				crash_policy: "destroy",
			}),
		},
	);

	if (!response.ok) {
		throw new Error(
			`Failed to create actor: ${response.status} ${response.statusText}\n${await response.text()}`,
		);
	}

	return response.json();
}

export async function destroyActor(
	namespaceName: string,
	actorId: string,
): Promise<undefined> {
	const response = await fetch(
		`${RIVET_ENDPOINT}/actors/${actorId}?namespace=${namespaceName}`,
		{
			method: "DELETE",
		},
	);

	if (!response.ok) {
		throw new Error(
			`Failed to delete actor: ${response.status} ${response.statusText}\n${await response.text()}`,
		);
	}
}

export async function getOrCreateActorById(
	namespaceName: string,
	name: string,
	key: string,
	runnerNameSelector: string,
): Promise<any> {
	const response = await fetch(
		`${RIVET_ENDPOINT}/actors/by-id?namespace=${namespaceName}`,
		{
			method: "PUT",
			headers: {
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				name,
				key,
				runner_name_selector: runnerNameSelector,
				crash_policy: "destroy",
			}),
		},
	);

	if (!response.ok) {
		throw new Error(
			`Failed to get or create actor: ${response.status} ${response.statusText}\n${await response.text()}`,
		);
	}

	return response.json();
}

export async function listActors(
	namespaceName: string,
	name?: string,
): Promise<any> {
	let url = `${RIVET_ENDPOINT}/actors?namespace=${namespaceName}`;
	if (name) {
		url += `&name=${name}`;
	}

	const response = await fetch(url, {
		method: "GET",
	});

	if (!response.ok) {
		throw new Error(
			`Failed to list actors: ${response.status} ${response.statusText}\n${await response.text()}`,
		);
	}

	return response.json();
}

export function generateRandomKey(): string {
	return `key-${Math.floor(Math.random() * 1000000)}`;
}

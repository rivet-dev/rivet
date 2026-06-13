import { randomUUID } from "node:crypto";
import {
	getOrStartSharedTestEngine,
	releaseSharedTestEngine,
	type SharedTestEngine,
	TEST_ENGINE_TOKEN,
} from "../../rivetkit/tests/shared-engine";

export { getOrStartSharedTestEngine, releaseSharedTestEngine, TEST_ENGINE_TOKEN };
export type { SharedTestEngine };

export interface PreparedNamespace {
	readonly endpoint: string;
	readonly token: string;
	readonly namespace: string;
	readonly poolName: string;
}

export async function prepareNamespace(
	endpoint: string,
	options: { namespace?: string; poolName?: string } = {},
): Promise<PreparedNamespace> {
	const namespace = options.namespace ?? `effect-e2e-${randomUUID()}`;
	const poolName = options.poolName ?? "default";
	await createNamespace(endpoint, namespace);
	await upsertNormalRunnerConfig(endpoint, namespace, poolName);
	return { endpoint, token: TEST_ENGINE_TOKEN, namespace, poolName };
}

async function createNamespace(
	endpoint: string,
	namespace: string,
): Promise<void> {
	const response = await fetch(`${endpoint}/namespaces`, {
		method: "POST",
		headers: {
			Authorization: `Bearer ${TEST_ENGINE_TOKEN}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			name: namespace,
			display_name: `Effect e2e ${namespace}`,
		}),
	});

	if (!response.ok) {
		throw new Error(
			`failed to create namespace ${namespace}: ${response.status} ${await response.text()}`,
		);
	}
}

export async function waitForEnvoy(
	endpoint: string,
	namespace: string,
	poolName: string,
	timeoutMs = 30_000,
): Promise<void> {
	const deadline = Date.now() + timeoutMs;

	while (Date.now() < deadline) {
		const response = await fetch(
			`${endpoint}/envoys?namespace=${encodeURIComponent(namespace)}&name=${encodeURIComponent(poolName)}`,
			{
				headers: {
					Authorization: `Bearer ${TEST_ENGINE_TOKEN}`,
				},
			},
		);

		if (response.ok) {
			const body = (await response.json()) as {
				envoys: Array<{ envoy_key: string }>;
			};
			if (body.envoys.length > 0) return;
		}

		await new Promise((resolve) => setTimeout(resolve, 250));
	}

	throw new Error(
		`timed out waiting for envoy in pool ${poolName} (namespace ${namespace})`,
	);
}

async function upsertNormalRunnerConfig(
	endpoint: string,
	namespace: string,
	poolName: string,
): Promise<void> {
	const datacentersResponse = await fetch(
		`${endpoint}/datacenters?namespace=${encodeURIComponent(namespace)}`,
		{
			headers: {
				Authorization: `Bearer ${TEST_ENGINE_TOKEN}`,
			},
		},
	);

	if (!datacentersResponse.ok) {
		throw new Error(
			`failed to list datacenters: ${datacentersResponse.status} ${await datacentersResponse.text()}`,
		);
	}

	const datacentersBody = (await datacentersResponse.json()) as {
		datacenters: Array<{ name: string }>;
	};
	const datacenter = datacentersBody.datacenters[0]?.name;

	if (!datacenter) {
		throw new Error("engine returned no datacenters");
	}

	const deadline = Date.now() + 30_000;

	while (Date.now() < deadline) {
		const response = await fetch(
			`${endpoint}/runner-configs/${encodeURIComponent(poolName)}?namespace=${encodeURIComponent(namespace)}`,
			{
				method: "PUT",
				headers: {
					Authorization: `Bearer ${TEST_ENGINE_TOKEN}`,
					"Content-Type": "application/json",
				},
				body: JSON.stringify({
					datacenters: {
						[datacenter]: {
							normal: {},
						},
					},
				}),
			},
		);

		if (response.ok) {
			return;
		}

		const responseBody = await response.text();
		// The engine briefly reports the just-created namespace as missing
		// or returns a transient internal_error before the create write
		// propagates. Match the driver harness pattern and retry both.
		if (
			(response.status === 400 &&
				responseBody.includes('"group":"namespace"') &&
				responseBody.includes('"code":"not_found"')) ||
			(response.status === 500 &&
				responseBody.includes('"group":"core"') &&
				responseBody.includes('"code":"internal_error"'))
		) {
			await new Promise((resolve) => setTimeout(resolve, 500));
			continue;
		}

		throw new Error(
			`failed to upsert runner config ${poolName}: ${response.status} ${responseBody}`,
		);
	}

	throw new Error(`timed out upserting runner config ${poolName}`);
}

import { describe, expect, test } from "vitest";
import { serializeActorKey } from "@/actor/keys";
import type { ActorsListResponse } from "@/manager-api/actors";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorMetadataTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Metadata Tests", () => {
		describe("Actor Name", () => {
			test("should provide access to actor name", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Get the actor name
				const handle = client.metadataActor.getOrCreate();
				const actorName = await handle.getActorName();

				// Verify it matches the expected name
				expect(actorName).toBe("metadataActor");
			});

			test("should preserve actor name in state during onWake", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Get the stored actor name
				const handle = client.metadataActor.getOrCreate();
				const storedName = await handle.getStoredActorName();

				// Verify it was stored correctly
				expect(storedName).toBe("metadataActor");
			});
		});

		describe("Actor Tags", () => {
			test("should provide access to tags", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and set up test tags
				const handle = client.metadataActor.getOrCreate();
				await handle.setupTestTags({
					env: "test",
					purpose: "metadata-test",
				});

				// Get the tags
				const tags = await handle.getTags();

				// Verify the tags are accessible
				expect(tags).toHaveProperty("env");
				expect(tags.env).toBe("test");
				expect(tags).toHaveProperty("purpose");
				expect(tags.purpose).toBe("metadata-test");
			});

			test("should allow accessing individual tags", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and set up test tags
				const handle = client.metadataActor.getOrCreate();
				await handle.setupTestTags({
					category: "test-actor",
					version: "1.0",
				});

				// Get individual tags
				const category = await handle.getTag("category");
				const version = await handle.getTag("version");
				const nonexistent = await handle.getTag("nonexistent");

				// Verify the tag values
				expect(category).toBe("test-actor");
				expect(version).toBe("1.0");
				expect(nonexistent).toBeNull();
			});
		});

		describe("Metadata Structure", () => {
			test("should provide complete metadata object", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and set up test metadata
				const handle = client.metadataActor.getOrCreate();
				await handle.setupTestTags({ type: "metadata-test" });
				await handle.setupTestRegion("us-west-1");

				// Get all metadata
				const metadata = await handle.getMetadata();

				// Verify structure of metadata
				expect(metadata).toHaveProperty("name");
				expect(metadata.name).toBe("metadataActor");

				expect(metadata).toHaveProperty("tags");
				expect(metadata.tags).toHaveProperty("type");
				expect(metadata.tags.type).toBe("metadata-test");

				// Region should be set to our test value
				expect(metadata).toHaveProperty("region");
				expect(metadata.region).toBe("us-west-1");
			});
		});

		describe("Region Information", () => {
			test("should retrieve region information", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and set up test region
				const handle = client.metadataActor.getOrCreate();
				await handle.setupTestRegion("eu-central-1");

				// Get the region
				const region = await handle.getRegion();

				// Verify the region is set correctly
				expect(region).toBe("eu-central-1");
			});
		});

		describe("Metadata Patching", () => {
			test("should patch metadata from inside the actor and return full metadata for actor_id lookups", async (c) => {
				const { client, endpoint, namespace } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = client.metadataActor.getOrCreate([
					`metadata-${crypto.randomUUID()}`,
				]);
				const actorId = await handle.resolve();

				await handle.patchMetadata({
					workflow_state: "failed",
					workflow_reason: "generate_report_failed",
				});

				const response = await fetchActors(
					endpoint,
					buildQueryParams({
						namespace,
						actor_ids: actorId,
					}),
				);
				const actor = response.actors.find(
					(candidate) => candidate.actor_id === actorId,
				);

				expect(actor?.metadata).toEqual({
					workflow_state: "failed",
					workflow_reason: "generate_report_failed",
				});
			});

			test("should patch metadata over rest and support overwrite and delete", async (c) => {
				const { client, endpoint, namespace } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const key = [`metadata-${crypto.randomUUID()}`];
				const handle = client.metadataActor.getOrCreate(key);
				const actorId = await handle.resolve();

				await sendMetadataPatch(endpoint, namespace, actorId, {
					workflow_state: "running",
					old_key: "stale",
				});
				await sendMetadataPatch(endpoint, namespace, actorId, {
					workflow_state: "failed",
					last_error: "timeout",
					old_key: null,
				});

				const response = await fetchActors(
					endpoint,
					buildQueryParams({
						namespace,
						name: "metadataActor",
						key: serializeActorKey(key),
					}),
				);

				expect(response.actors[0]?.metadata).toEqual({
					workflow_state: "failed",
					last_error: "timeout",
				});
			});

			test("should omit metadata by default and project only requested keys for list queries", async (c) => {
				const { client, endpoint, namespace } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = client.metadataActor.getOrCreate([
					`metadata-${crypto.randomUUID()}`,
				]);
				const actorId = await handle.resolve();

				await sendMetadataPatch(endpoint, namespace, actorId, {
					workflow_state: "failed",
					last_error: "timeout",
				});

				const unprojected = await fetchActors(
					endpoint,
					buildQueryParams({
						namespace,
						name: "metadataActor",
					}),
				);
				expect(findActor(unprojected, actorId)?.metadata).toBeUndefined();

				const projected = await fetchActors(
					endpoint,
					buildQueryParams(
						{
							namespace,
							name: "metadataActor",
						},
						["workflow_state", "missing_key"],
					),
				);
				expect(findActor(projected, actorId)?.metadata).toEqual({
					workflow_state: "failed",
				});

				const emptyProjection = await fetchActors(
					endpoint,
					buildQueryParams(
						{
							namespace,
							name: "metadataActor",
						},
						["missing_key"],
					),
				);
				expect(findActor(emptyProjection, actorId)?.metadata).toEqual({});
			});

			test("should reject list queries with too many metadata keys", async (c) => {
				const { endpoint, namespace } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const response = await fetch(
					buildManagerUrl(
						endpoint,
						"/actors",
						buildQueryParams(
							{
								namespace,
								name: "metadataActor",
							},
							Array.from({ length: 17 }, (_, i) => `k${i}`),
						),
					),
				);

				expect(response.status).toBe(400);
			});
		});
	});
}

function buildQueryParams(
	values: Record<string, string>,
	metadataKeys: string[] = [],
): URLSearchParams {
	const query = new URLSearchParams(values);
	for (const metadataKey of metadataKeys) {
		query.append("metadata_key", metadataKey);
	}
	return query;
}

function buildManagerUrl(
	endpoint: string,
	path: string,
	query?: URLSearchParams,
): string {
	const normalizedEndpoint = endpoint.endsWith("/")
		? endpoint.slice(0, -1)
		: endpoint;
	return `${normalizedEndpoint}${path}${query ? `?${query.toString()}` : ""}`;
}

async function fetchActors(
	endpoint: string,
	query: URLSearchParams,
): Promise<ActorsListResponse> {
	const response = await fetch(buildManagerUrl(endpoint, "/actors", query));
	expect(response.ok).toBe(true);
	return (await response.json()) as ActorsListResponse;
}

async function sendMetadataPatch(
	endpoint: string,
	namespace: string,
	actorId: string,
	metadata: Record<string, string | null>,
): Promise<void> {
	const response = await fetch(
		buildManagerUrl(
			endpoint,
			`/actors/${encodeURIComponent(actorId)}/metadata`,
			buildQueryParams({ namespace }),
		),
		{
			method: "PATCH",
			headers: {
				"content-type": "application/json",
			},
			body: JSON.stringify({ metadata }),
		},
	);

	expect(response.ok).toBe(true);
}

function findActor(
	response: ActorsListResponse,
	actorId: string,
) {
	return response.actors.find((actor) => actor.actor_id === actorId);
}

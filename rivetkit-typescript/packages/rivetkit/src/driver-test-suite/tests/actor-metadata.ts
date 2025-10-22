import { describe, expect, test } from "vitest";
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

			test("should preserve actor name in state during onStart", async (c) => {
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
	});
}

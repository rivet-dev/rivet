import { describe, expect, it } from "vitest";
import {
	CLUSTER_CONFIG_FILENAME,
	serializeClusterConfig,
} from "./cluster-config";
const id = "10000000-0000-4000-8000-000000000001";
describe("BYOC cluster configuration", () => {
	it("downloads only the cluster ID for production", () => {
		expect(CLUSTER_CONFIG_FILENAME).toBe("rivet.auto.tfvars.json");
		expect(
			JSON.parse(
				serializeClusterConfig(id, "https://cloud-api.rivet.dev/")!,
			),
		).toEqual({ byoc_cluster_id: id });
	});
	it("includes a non-production origin", () => {
		expect(
			JSON.parse(
				serializeClusterConfig(
					id,
					"https://cloud-api.staging.rivet.dev",
				)!,
			),
		).toEqual({
			byoc_cluster_id: id,
			cloud_api_url: "https://cloud-api.staging.rivet.dev",
		});
	});
	it("rejects incomplete identity and unsafe origins", () => {
		expect(
			serializeClusterConfig(undefined, "https://cloud-api.rivet.dev"),
		).toBeNull();
		expect(
			serializeClusterConfig("slug", "https://cloud-api.rivet.dev"),
		).toBeNull();
		for (const url of [
			"http://cloud.example.com",
			"https://user:secret@cloud.example.com",
			"https://cloud.example.com/path",
			"https://cloud.example.com/?token=secret",
			"https://cloud.example.com/#secret",
			"invalid",
		])
			expect(serializeClusterConfig(id, url)).toBeNull();
	});
});

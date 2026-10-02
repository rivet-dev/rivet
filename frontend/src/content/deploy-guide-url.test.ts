import {
	deployOptions,
	WORKER_DEPLOY_GUIDES_URL,
	workerDeployGuideUrl,
} from "@rivetkit/shared-data";
import { describe, expect, it } from "vitest";

describe("workerDeployGuideUrl", () => {
	it("uses the guide slug, which can differ from the provider name", () => {
		expect(workerDeployGuideUrl("cloudflare-workers")).toBe(
			"https://rivet.dev/docs/deploy/self-host/workers/cloudflare",
		);
		expect(workerDeployGuideUrl("supabase-functions")).toBe(
			"https://rivet.dev/docs/deploy/self-host/workers/supabase",
		);
	});

	it("builds a guide URL for every deploy option", () => {
		for (const option of deployOptions) {
			expect(workerDeployGuideUrl(option.name)).toBe(
				`${WORKER_DEPLOY_GUIDES_URL}${option.slug}`,
			);
		}
	});

	it("sends Hetzner to the VM guide", () => {
		expect(workerDeployGuideUrl("hetzner")).toBe(
			"https://rivet.dev/docs/deploy/self-host/workers/vm",
		);
	});

	it("falls back to the guide index for unknown or missing providers", () => {
		expect(workerDeployGuideUrl("rivet")).toBe(WORKER_DEPLOY_GUIDES_URL);
		expect(workerDeployGuideUrl(undefined)).toBe(WORKER_DEPLOY_GUIDES_URL);
	});
});

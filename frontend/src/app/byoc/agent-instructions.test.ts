import { expect, it } from "vitest";
import { serializeAgentInstructions } from "./agent-instructions";
const id = "10000000-0000-4000-8000-000000000001";
const token = "test-operator-token-" + "a".repeat(40);
it("copies setup URLs, exact cluster config, and token with secret handling instructions", () => {
	const text = serializeAgentInstructions(
		id,
		"https://cloud-api.staging.rivet.dev",
		token,
	)!;
	expect(text).toContain("https://rivet.dev/cloud/byoc/quickstart/");
	expect(text).toContain(
		"https://releases.rivet.dev/byoc/latest/setup-kit.tar.gz",
	);
	const config = JSON.parse(text.split("```json\n")[1].split("\n```")[0]);
	expect(config).toEqual({
		byoc_cluster_id: id,
		cloud_api_url: "https://cloud-api.staging.rivet.dev",
	});
	expect(JSON.stringify(config)).not.toContain(token);
	expect(text.split(token)).toHaveLength(2);
	expect(text).toContain("RIVET_BYOC_OPERATOR_TOKEN");
	expect(text).toContain("Do not repeat it in your response");
	expect(text).toContain("rivet.auto.tfvars.json");
	expect(text).toContain("Terraform process environment");
	expect(text).toContain("do not ask me to download config or copy a token");
	expect(text).not.toContain("## Supplied cluster config");
	expect(text).not.toContain("rivet-credentials.json");
	expect(text.indexOf("```json")).toBeGreaterThan(
		text.indexOf("## 3. Prepare and install"),
	);
	expect(text.indexOf("RIVET_BYOC_OPERATOR_TOKEN")).toBeLessThan(
		text.indexOf("Show me the Terraform plan"),
	);
});
it("asks for basic configuration before provisioning and defines safe completion", () => {
	const text = serializeAgentInstructions(
		id,
		"https://cloud-api.rivet.dev",
		token,
	)!;
	expect(text.indexOf("## 2. Ask me")).toBeLessThan(
		text.indexOf("## 3. Prepare and install"),
	);
	for (const field of [
		"name",
		"regions",
		"leader",
		"rivet_endpoint_access",
		"rivet_hostname",
		"kubernetes_namespace",
		"kubernetes_operator_namespace",
	])
		expect(text).toContain(field);
	expect(text).toContain("ask only for public access");
	expect(text).toContain("wait for my confirmation rather than guessing");
	expect(text).toContain("get approval before applying");
	expect(text).toContain(
		"same extracted kit, Terraform directory, and state",
	);
	expect(text).toContain("show me the exact records to create");
	expect(text).toContain("operator is healthy and connected to Rivet Cloud");
	expect(text).toContain(
		"Provisioning the operator does not deploy the Engine",
	);
	expect(text).not.toContain("Clear the environment variable");
});

it("does not copy instructions with missing or malformed credentials", () => {
	for (const value of [
		undefined,
		"",
		"short",
		token + "\n",
		"x".repeat(16385),
	]) {
		expect(
			serializeAgentInstructions(
				id,
				"https://cloud-api.rivet.dev",
				value,
			),
		).toBeNull();
	}
	expect(
		serializeAgentInstructions(
			undefined,
			"https://cloud-api.rivet.dev",
			token,
		),
	).toBeNull();
	expect(
		serializeAgentInstructions(id, "http://cloud.example.com", token),
	).toBeNull();
});

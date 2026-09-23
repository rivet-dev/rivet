import { randomUUID } from "node:crypto";
import { fauxAssistantMessage } from "@earendil-works/pi-ai";
import { SpanStatusCode } from "@opentelemetry/api";
import {
	InMemorySpanExporter,
	NodeTracerProvider,
	type ReadableSpan,
	SimpleSpanProcessor,
} from "@opentelemetry/sdk-trace-node";
import { setup } from "rivetkit";
import { setupTest } from "rivetkit/test";
import { afterAll, beforeAll, describe, expect, test, vi } from "vitest";
import { pi } from "../src/index.js";
import { type MockModel, slowly, startMockModel } from "./helpers/mock-model.js";
import { type OtlpCollector, startOtlpCollector } from "./helpers/otlp-collector.js";

const piSpans = new InMemorySpanExporter();
new NodeTracerProvider({
	spanProcessors: [new SimpleSpanProcessor(piSpans)],
}).register();

let collector: OtlpCollector;
let mockModel: MockModel;
let registry: ReturnType<typeof buildRegistry>;

function buildRegistry(mock: MockModel) {
	const agent = pi({
		model: mock.model,
		modelRuntime: mock.modelRuntime,
		settings: { retry: { baseDelayMs: 10 } },
	});
	return setup({ use: { agent } });
}

beforeAll(async () => {
	collector = await startOtlpCollector();
	process.env.OTEL_EXPORTER_OTLP_TRACES_ENDPOINT = collector.endpoint;
	process.env.OTEL_EXPORTER_OTLP_TRACES_PROTOCOL = "http/json";
	process.env.OTEL_TRACES_SAMPLER = "always_on";
	process.env.OTEL_BSP_SCHEDULE_DELAY = "10";

	mockModel = await startMockModel();
	registry = buildRegistry(mockModel);

	mockModel.reply(
		"fail at the model",
		fauxAssistantMessage("", { stopReason: "error", errorMessage: "400 credit balance is too low" }),
	);
	mockModel.reply("answer after a pause", slowly(fauxAssistantMessage("done")));
	mockModel.reply(
		"answer despite overload",
		fauxAssistantMessage("", { stopReason: "error", errorMessage: "503 overloaded" }),
		fauxAssistantMessage("recovered"),
	);
});

afterAll(async () => {
	await mockModel?.dispose();
	await collector?.close();
});

function runSpans(): ReadableSpan[] {
	return piSpans.getFinishedSpans().filter((span) => span.name === "invoke_agent pi");
}

function lastRun(): ReadableSpan {
	const run = runSpans().at(-1);
	expect(run).toBeDefined();
	return run!;
}

/** Checks that the run span is a child of the same actor's `prompt` action span. */
async function expectUnderPromptAction(run: ReadableSpan): Promise<void> {
	// RivetKit exports the action span in batches, so wait for it to arrive.
	await vi.waitFor(() => {
		const action = collector.spans().find((span) => span.spanId === run.parentSpanContext?.spanId);
		expect(action?.attributes["rivet.action.name"]).toBe("prompt");
		expect(action?.attributes["rivet.actor.id"]).toBe(run.attributes["rivet.actor.id"]);
		expect(action?.traceId).toBe(run.spanContext().traceId);
	});
}

describe("pi run tracing", () => {
	test("a run that fails at the model is a failed run span under the prompt action", async (c) => {
		piSpans.reset();
		const { client } = await setupTest(c, registry);
		const handle = client.agent.getOrCreate(["model-failure", randomUUID()]);

		await expect(handle.prompt("fail at the model")).resolves.toBeUndefined();

		const run = lastRun();
		expect(run.status.code).toBe(SpanStatusCode.ERROR);
		expect(run.status.message).toContain("credit balance is too low");
		expect(run.attributes["error.type"]).toBe("model_error");
		await expectUnderPromptAction(run);
	});

	test("a run whose failed model call Pi retried successfully is not a failed run", async (c) => {
		piSpans.reset();
		const { client } = await setupTest(c, registry);
		const handle = client.agent.getOrCreate(["retried", randomUUID()]);

		await handle.prompt("answer despite overload");

		expect(lastRun().status.code).not.toBe(SpanStatusCode.ERROR);
	});

	test("runs of two actors prompted at the same time stay in their own traces", async (c) => {
		piSpans.reset();
		const { client } = await setupTest(c, registry);
		const handles = [randomUUID(), randomUUID()].map((id) => client.agent.getOrCreate(["overlap", id]));

		await Promise.all(handles.map((handle) => handle.prompt("answer after a pause")));

		const runs = runSpans();
		expect(runs).toHaveLength(2);
		expect(new Set(runs.map((run) => run.spanContext().traceId)).size).toBe(2);
		for (const run of runs) await expectUnderPromptAction(run);
	});
});

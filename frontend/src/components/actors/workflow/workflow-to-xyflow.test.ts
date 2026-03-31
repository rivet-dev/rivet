import { describe, expect, it } from "vitest";
import { tryWorkflow } from "./workflow-example-data";
import { workflowHistoryToXYFlow } from "./workflow-to-xyflow";

describe("workflowHistoryToXYFlow", () => {
	it("renders synthetic try scopes instead of dropping nested try entries", () => {
		const { nodes } = workflowHistoryToXYFlow(tryWorkflow);

		expect(
			nodes.some(
				(node) =>
					node.type === "tryGroup" &&
					node.data?.label === "payment-flow",
			),
		).toBe(true);

		expect(
			nodes.some(
				(node) =>
					node.type === "workflow" &&
					node.data?.label === "parallel-verification",
			),
		).toBe(true);
	});

	it("marks handled step failures so tryStep is visually distinct", () => {
		const { nodes } = workflowHistoryToXYFlow(tryWorkflow);
		const chargeCardNode = nodes.find(
			(node) =>
				node.type === "workflow" && node.data?.label === "charge-card",
		);
		const reserveStockNode = nodes.find(
			(node) =>
				node.type === "workflow" &&
				node.data?.label === "reserve-stock",
		);

		expect(chargeCardNode?.data?.handledFailure).toBe(true);
		expect(chargeCardNode?.data?.summary).toBe("handled error");
		expect(reserveStockNode?.data?.handledFailure).toBe(true);
	});
});

import { faSpinnerThird, Icon } from "@rivet-gg/icons";
import type { PropsWithChildren } from "react";
import { WorkflowVisualizer } from "./workflow-visualizer";
import type { WorkflowHistory } from "./workflow-types";

interface ActorWorkflowTabProps {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	actorId: string;
}

// Sample workflow data for demonstration
// This will be replaced with actual API data when the workflow inspector is implemented
const sampleWorkflowHistory: WorkflowHistory = {
	workflowId: "d579cc14-f798-42de-b4b6-2d10fa37d03b",
	state: "completed",
	nameRegistry: [
		"bootstrap",
		"validate-input",
		"checkpoint-after-validation",
		"load-user-profile",
		"compute-discount",
		"ephemeral-cache-check",
		"checkpoint-before-reserve",
		"process-items-loop",
		"fetch-item-0",
		"compute-tax-0",
		"reserve-inventory-0",
		"fetch-item-1",
		"compute-tax-1",
		"reserve-inventory-1",
		"short-cooldown",
		"cooldown-sleep",
		"join-dependencies",
		"inventory",
		"inventory-audit",
		"pricing",
		"pricing-method",
		"shipping",
		"shipping-zone",
		"join-inventory-sleep",
		"join-shipping-sleep",
		"race-fulfillment",
		"race-fast",
		"race-fast-sleep",
		"race-slow",
		"race-slow-sleep",
		"finalize",
	],
	input: {
		userId: "user-123",
		itemIds: ["item-1", "item-2", "item-3", "item-4"],
		deadlineMs: 1769561520725,
	},
	output: {
		workflowId: "0f6fe6cf-e6ca-46de-9512-72db613c2ad6",
		bootstrap: {
			requestId: "3958c87d-9dcb-4f20-8795-6317794e4351",
			startedAt: 1769561520421,
		},
		profile: {
			id: "user-123",
			tier: "standard",
			flags: ["email-verified", "promo-eligible"],
		},
		discount: { percent: 5, reason: "tier-discount" },
		items: {
			receipts: [
				{
					itemId: "item-1",
					basePrice: 100,
					tax: 8,
					finalPrice: 103,
					reservationId: "402885a5-2b88-423f-8500-6eb641073a5f",
				},
				{
					itemId: "item-2",
					basePrice: 115,
					tax: 9,
					finalPrice: 118,
					reservationId: "05055094-3c17-4527-9374-a16abd2a0ff6",
				},
			],
			summary: { count: 4, total: 504 },
		},
	},
	history: [
		{
			key: "bootstrap",
			entry: {
				id: "26299b0e-70e8-4b30-a8f2-c452775b224c",
				location: [0],
				kind: {
					type: "step",
					data: {
						output: {
							requestId: "97531aad-6075-47ea-90f1-31c19262b750",
							startedAt: 1769562508317,
						},
					},
				},
				dirty: false,
				startedAt: 1769562508317,
				completedAt: 1769562508350,
			},
		},
		{
			key: "validate-input",
			entry: {
				id: "c4668f41-8d82-4510-9b39-df2c107463d3",
				location: [1],
				kind: {
					type: "step",
					data: { output: true },
				},
				dirty: false,
				startedAt: 1769562508355,
				completedAt: 1769562508412,
			},
		},
		{
			key: "checkpoint-after-validation",
			entry: {
				id: "a4569615-5446-4ca0-85fc-f41a1502ea4a",
				location: [2],
				kind: {
					type: "rollback_checkpoint",
					data: { name: "checkpoint-after-validation" },
				},
				dirty: false,
				startedAt: 1769562508420,
				completedAt: 1769562508425,
			},
		},
		{
			key: "load-user-profile",
			entry: {
				id: "6eee86bd-3820-4ff5-ab6f-f54af37316d9",
				location: [3],
				kind: {
					type: "step",
					data: {
						output: {
							id: "user-123",
							tier: "standard",
							flags: ["email-verified", "promo-eligible"],
						},
					},
				},
				dirty: false,
				startedAt: 1769562508430,
				completedAt: 1769562508892,
			},
		},
		{
			key: "compute-discount",
			entry: {
				id: "bb09df91-3e00-4138-8a2f-529be262091d",
				location: [4],
				kind: {
					type: "step",
					data: { output: { percent: 5, reason: "tier-discount" } },
				},
				dirty: false,
				startedAt: 1769562508900,
				completedAt: 1769562508945,
			},
		},
		{
			key: "ephemeral-cache-check",
			entry: {
				id: "e7ebd9ed-d71d-4f75-914b-dafd7e009087",
				location: [5],
				kind: {
					type: "step",
					data: { output: { cacheHit: false, tier: "standard" } },
				},
				dirty: false,
				startedAt: 1769562508950,
				completedAt: 1769562509012,
			},
		},
		{
			key: "short-cooldown",
			entry: {
				id: "cd8929e1-6e36-4fdc-bee6-ef9154e32433",
				location: [6],
				kind: {
					type: "sleep",
					data: { deadline: 1769562508608, state: "completed" },
				},
				dirty: false,
				startedAt: 1769562508500,
				completedAt: 1769562508608,
			},
		},
		{
			key: "join-dependencies",
			entry: {
				id: "ea93a137-db66-48fb-b1fd-9e6f5d5a084c",
				location: [7],
				kind: {
					type: "join",
					data: {
						branches: {
							inventory: {
								status: "completed",
								output: {
									reserved: 4,
									checked: 4,
									notes: ["inventory-ok", "items=4"],
								},
							},
							pricing: {
								status: "completed",
								output: {
									subtotal: 504,
									discount: 25,
									total: 479,
									method: "promo",
								},
							},
							shipping: {
								status: "completed",
								output: { method: "ground", etaDays: 4, zone: "us-east" },
							},
						},
					},
				},
				dirty: false,
				startedAt: 1769562509000,
				completedAt: 1769562509500,
			},
		},
		{
			key: "join-dependencies/inventory/inventory-audit",
			entry: {
				id: "7ac02dbe-5dc0-45cb-8c91-bbb102962a1a",
				location: [7, 8, 9],
				kind: {
					type: "step",
					data: { output: 4 },
				},
				dirty: false,
				startedAt: 1769562509050,
				completedAt: 1769562509150,
			},
		},
		{
			key: "join-dependencies/inventory/join-inventory-sleep",
			entry: {
				id: "9390eac8-5a96-47a9-8e9c-bc2618c1e5f9",
				location: [7, 8, 10],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509509, state: "completed" },
				},
				dirty: false,
				startedAt: 1769562509160,
				completedAt: 1769562509400,
			},
		},
		{
			key: "join-dependencies/pricing/pricing-method",
			entry: {
				id: "7a6047fc-4fd6-4945-89a5-95735b5609d6",
				location: [7, 11, 12],
				kind: {
					type: "step",
					data: { output: "promo" },
				},
				dirty: false,
				startedAt: 1769562509050,
				completedAt: 1769562509200,
			},
		},
		{
			key: "join-dependencies/shipping/shipping-zone",
			entry: {
				id: "0603cbf7-bc00-4ad1-8477-1558d6924e15",
				location: [7, 13, 14],
				kind: {
					type: "step",
					data: { output: "us-east" },
				},
				dirty: false,
				startedAt: 1769562509050,
				completedAt: 1769562509180,
			},
		},
		{
			key: "join-dependencies/shipping/join-shipping-sleep",
			entry: {
				id: "8de4fa8f-77d5-41c3-b73a-00ad4c66548b",
				location: [7, 13, 15],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509509, state: "completed" },
				},
				dirty: false,
				startedAt: 1769562509190,
				completedAt: 1769562509450,
			},
		},
		{
			key: "race-fulfillment",
			entry: {
				id: "623243da-8b14-4b3b-9a83-aa4bdd1a4e43",
				location: [16],
				kind: {
					type: "race",
					data: {
						winner: "race-fast",
						branches: {
							"race-fast": {
								status: "completed",
								output: { method: "express", cost: 18, etaDays: 1 },
							},
							"race-slow": {
								status: "cancelled",
								error: "Cancelled: lost race",
							},
						},
					},
				},
				dirty: false,
				startedAt: 1769562509520,
				completedAt: 1769562509700,
			},
		},
		{
			key: "race-fulfillment/race-fast/race-fast-sleep",
			entry: {
				id: "74dc4690-7567-4dee-9067-33d7d9d8b9ef",
				location: [16, 17, 18],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509643, state: "completed" },
				},
				dirty: false,
				startedAt: 1769562509550,
				completedAt: 1769562509650,
			},
		},
		{
			key: "finalize",
			entry: {
				id: "d9da246b-54c7-4681-afef-3eda5c2f66c0",
				location: [19],
				kind: {
					type: "step",
					data: { output: true },
				},
				dirty: false,
				startedAt: 1769562509750,
				completedAt: 1769562509800,
			},
		},
	],
};

export function ActorWorkflowTab(_props: ActorWorkflowTabProps) {
	// For now, show sample workflow data
	// In the future, this will use the inspector API to get real workflow data
	const isWorkflowEnabled = true;
	const isLoading = false;
	const isError = false;

	if (isError) {
		return (
			<Info>
				Workflow Visualizer is currently unavailable.
				<br />
				See console/logs for more details.
			</Info>
		);
	}

	if (isLoading) {
		return (
			<Info>
				<div className="flex items-center">
					<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
					Loading Workflow...
				</div>
			</Info>
		);
	}

	if (!isWorkflowEnabled) {
		return (
			<Info>
				<p>
					Workflow Visualizer is not enabled for this Actor. <br /> This
					feature requires a workflow-based Actor.
				</p>
			</Info>
		);
	}

	return (
		<div className="flex-1 w-full min-h-0 h-full flex flex-col">
			<WorkflowVisualizer workflow={sampleWorkflowHistory} />
		</div>
	);
}

function Info({ children }: PropsWithChildren) {
	return (
		<div className="flex-1 flex flex-col gap-2 items-center justify-center h-full text-center max-w-lg mx-auto">
			{children}
		</div>
	);
}

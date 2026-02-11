import type { WorkflowHistory } from "./workflow-types";

// Simple linear workflow with timing
export const simpleLinearWorkflow: WorkflowHistory = {
	workflowId: "simple-linear-001",
	state: "completed",
	nameRegistry: ["start", "process", "validate", "complete"],
	input: { userId: "user-123", action: "processData", items: [1, 2, 3] },
	output: { success: true, processedItems: 3, duration: "2.62s" },
	history: [
		{
			key: "start",
			entry: {
				id: "1",
				location: [0],
				kind: {
					type: "step",
					data: { output: { initialized: true } },
				},
				dirty: false,
				startedAt: 1700000000000,
				completedAt: 1700000000050,
			},
		},
		{
			key: "process",
			entry: {
				id: "2",
				location: [1],
				kind: {
					type: "step",
					data: { output: { processed: true, items: 5 } },
				},
				dirty: false,
				startedAt: 1700000000750,
				completedAt: 1700000001010,
			},
		},
		{
			key: "validate",
			entry: {
				id: "3",
				location: [2],
				kind: { type: "step", data: { output: { valid: true } } },
				dirty: false,
				startedAt: 1700000003200,
				completedAt: 1700000003280,
			},
		},
		{
			key: "complete",
			entry: {
				id: "4",
				location: [3],
				kind: { type: "step", data: { output: { success: true } } },
				dirty: false,
				startedAt: 1700000003880,
				completedAt: 1700000003910,
			},
		},
	],
};

// Workflow with a loop
export const loopWorkflow: WorkflowHistory = {
	workflowId: "loop-workflow-002",
	state: "completed",
	nameRegistry: [
		"init",
		"batch-loop",
		"process-0",
		"process-1",
		"process-2",
		"finalize",
	],
	history: [
		{
			key: "init",
			entry: {
				id: "1",
				location: [0],
				kind: {
					type: "step",
					data: { output: { batchSize: 3 } },
				},
				dirty: false,
				startedAt: 1700000100000,
				completedAt: 1700000100025,
			},
		},
		{
			key: "batch-loop",
			entry: {
				id: "2",
				location: [1],
				kind: {
					type: "loop",
					data: {
						state: { index: 3 },
						iteration: 3,
						output: { processed: 3 },
					},
				},
				dirty: false,
				startedAt: 1700000100030,
				completedAt: 1700000100850,
			},
		},
		{
			key: "batch-loop/~0/process-0",
			entry: {
				id: "3",
				location: [1, { loop: 1, iteration: 0 }, 2],
				kind: {
					type: "step",
					data: { output: { item: "A", status: "done" } },
				},
				dirty: false,
				startedAt: 1700000100035,
				completedAt: 1700000100280,
			},
		},
		{
			key: "batch-loop/~1/process-1",
			entry: {
				id: "4",
				location: [1, { loop: 1, iteration: 1 }, 3],
				kind: {
					type: "step",
					data: { output: { item: "B", status: "done" } },
				},
				dirty: false,
				startedAt: 1700000100285,
				completedAt: 1700000100560,
			},
		},
		{
			key: "batch-loop/~2/process-2",
			entry: {
				id: "5",
				location: [1, { loop: 1, iteration: 2 }, 4],
				kind: {
					type: "step",
					data: { output: { item: "C", status: "done" } },
				},
				dirty: false,
				startedAt: 1700000100565,
				completedAt: 1700000100845,
			},
		},
		{
			key: "finalize",
			entry: {
				id: "6",
				location: [5],
				kind: {
					type: "step",
					data: { output: { allProcessed: true } },
				},
				dirty: false,
				startedAt: 1700000100850,
				completedAt: 1700000100890,
			},
		},
	],
};

// Workflow with join (parallel branches)
export const joinWorkflow: WorkflowHistory = {
	workflowId: "join-workflow-003",
	state: "completed",
	nameRegistry: [
		"start",
		"parallel-tasks",
		"task-a",
		"task-b",
		"task-c",
		"merge-results",
	],
	history: [
		{
			key: "start",
			entry: {
				id: "1",
				location: [0],
				kind: {
					type: "step",
					data: { output: { ready: true } },
				},
				dirty: false,
				startedAt: 1700000200000,
				completedAt: 1700000200035,
			},
		},
		{
			key: "parallel-tasks",
			entry: {
				id: "2",
				location: [1],
				kind: {
					type: "join",
					data: {
						branches: {
							"fetch-api": {
								status: "completed",
								output: { data: "api-response" },
							},
							"query-db": {
								status: "completed",
								output: { rows: 42 },
							},
							"check-cache": {
								status: "completed",
								output: { hit: true },
							},
						},
					},
				},
				dirty: false,
				startedAt: 1700000200040,
				completedAt: 1700000200520,
			},
		},
		{
			key: "parallel-tasks/fetch-api/task-a",
			entry: {
				id: "3",
				location: [1, 2, 3],
				kind: {
					type: "step",
					data: { output: { fetched: true } },
				},
				dirty: false,
				startedAt: 1700000200045,
				completedAt: 1700000200320,
			},
		},
		{
			key: "parallel-tasks/query-db/task-b",
			entry: {
				id: "4",
				location: [1, 4, 5],
				kind: {
					type: "step",
					data: { output: { queried: true } },
				},
				dirty: false,
				startedAt: 1700000200045,
				completedAt: 1700000200510,
			},
		},
		{
			key: "parallel-tasks/check-cache/task-c",
			entry: {
				id: "5",
				location: [1, 6, 7],
				kind: {
					type: "step",
					data: { output: { checked: true } },
				},
				dirty: false,
				startedAt: 1700000200045,
				completedAt: 1700000200125,
			},
		},
		{
			key: "merge-results",
			entry: {
				id: "6",
				location: [8],
				kind: {
					type: "step",
					data: { output: { merged: true } },
				},
				dirty: false,
				startedAt: 1700000200525,
				completedAt: 1700000200580,
			},
		},
	],
};

// Workflow with race
export const raceWorkflow: WorkflowHistory = {
	workflowId: "race-workflow-004",
	state: "completed",
	nameRegistry: [
		"begin",
		"race-providers",
		"provider-fast",
		"provider-slow",
		"use-result",
	],
	input: { query: "fetch data", timeout: 5000 },
	output: { result: "fast response", provider: "provider-fast" },
	history: [
		{
			key: "begin",
			entry: {
				id: "1",
				location: [0],
				kind: {
					type: "step",
					data: { output: { started: true } },
				},
				dirty: false,
				startedAt: 1700000300000,
				completedAt: 1700000300020,
			},
		},
		{
			key: "race-providers",
			entry: {
				id: "2",
				location: [1],
				kind: {
					type: "race",
					data: {
						winner: "provider-fast",
						branches: {
							"provider-fast": {
								status: "completed",
								output: {
									provider: "cdn-edge",
									latency: 12,
								},
							},
							"provider-slow": {
								status: "cancelled",
								error: "Cancelled: lost race",
							},
						},
					},
				},
				dirty: false,
				startedAt: 1700000300520,
				completedAt: 1700000301145,
			},
		},
		{
			key: "race-providers/provider-fast/provider-fast-step",
			entry: {
				id: "3",
				location: [1, 2, 3],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509000, state: "completed" },
				},
				dirty: false,
				startedAt: 1700000300650,
				completedAt: 1700000301100,
			},
		},
		{
			key: "use-result",
			entry: {
				id: "4",
				location: [4],
				kind: {
					type: "step",
					data: { output: { used: "cdn-edge" } },
				},
				dirty: false,
				startedAt: 1700000301850,
				completedAt: 1700000301885,
			},
		},
	],
};

// Full workflow
export const sampleWorkflowHistory: WorkflowHistory = {
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
		"fetch-item-2",
		"compute-tax-2",
		"reserve-inventory-2",
		"fetch-item-3",
		"compute-tax-3",
		"reserve-inventory-3",
		"short-cooldown",
		"cooldown-sleep",
		"wait-until-deadline",
		"compute-deadlines",
		"listen-order-created:count",
		"listen-order-created:0",
		"listen-order-updated-timeout",
		"listen-order-updated-timeout:message",
		"listen-batch-two:count",
		"listen-batch-two:0",
		"listen-batch-two:1",
		"listen-artifacts-timeout:deadline",
		"listen-artifacts-timeout:count",
		"listen-artifacts-timeout:0",
		"listen-artifacts-timeout:1",
		"listen-artifacts-timeout:2",
		"listen-optional",
		"listen-optional:message",
		"listen-until",
		"listen-until:message",
		"listen-batch-until:count",
		"listen-batch-until:0",
		"listen-batch-until:1",
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
		"legacy-step-placeholder",
		"finalize",
	],
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
					data: {
						output: { percent: 5, reason: "tier-discount" },
					},
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
					data: {
						output: { cacheHit: false, tier: "standard" },
					},
				},
				dirty: false,
				startedAt: 1769562508950,
				completedAt: 1769562509012,
			},
		},
		{
			key: "checkpoint-before-reserve",
			entry: {
				id: "2cdcf051-afa7-4147-a146-e09f099e23ed",
				location: [6],
				kind: {
					type: "rollback_checkpoint",
					data: { name: "checkpoint-before-reserve" },
				},
				dirty: false,
				startedAt: 1769562509015,
				completedAt: 1769562509020,
			},
		},
		{
			key: "process-items-loop",
			entry: {
				id: "97f40f5e-3dc5-43e0-a123-bed5666b5590",
				location: [7],
				kind: {
					type: "loop",
					data: {
						state: {
							index: 4,
							count: 4,
							total: 504,
							receipts: [],
						},
						iteration: 4,
						output: { count: 4, total: 504, receipts: [] },
					},
				},
				dirty: false,
			},
		},
		{
			key: "process-items-loop/~2/fetch-item-2",
			entry: {
				id: "b8733cb5-8e57-4c62-9f3c-d2235060c58a",
				location: [7, { loop: 7, iteration: 2 }, 14],
				kind: {
					type: "step",
					data: {
						output: { itemId: "item-3", basePrice: 130 },
					},
				},
				dirty: false,
			},
		},
		{
			key: "process-items-loop/~2/compute-tax-2",
			entry: {
				id: "4c279a9c-fe62-4cb7-851a-36e610081ffb",
				location: [7, { loop: 7, iteration: 2 }, 15],
				kind: {
					type: "step",
					data: { output: 10 },
				},
				dirty: false,
			},
		},
		{
			key: "process-items-loop/~2/reserve-inventory-2",
			entry: {
				id: "b9547c93-a88f-4feb-8984-68550e156fb3",
				location: [7, { loop: 7, iteration: 2 }, 16],
				kind: {
					type: "step",
					data: {
						output: {
							reservationId:
								"472815f7-2f9a-4057-99eb-2ebf02d8044c",
							itemId: "item-3",
						},
					},
				},
				dirty: false,
			},
		},
		{
			key: "process-items-loop/~3/fetch-item-3",
			entry: {
				id: "e66a6b1f-7794-432b-abad-a7a8260d9f43",
				location: [7, { loop: 7, iteration: 3 }, 17],
				kind: {
					type: "step",
					data: {
						output: { itemId: "item-4", basePrice: 145 },
					},
				},
				dirty: false,
			},
		},
		{
			key: "process-items-loop/~3/compute-tax-3",
			entry: {
				id: "d45885d1-b7ae-4f5b-9108-bf365a952ec5",
				location: [7, { loop: 7, iteration: 3 }, 18],
				kind: {
					type: "step",
					data: { output: 12 },
				},
				dirty: false,
			},
		},
		{
			key: "process-items-loop/~3/reserve-inventory-3",
			entry: {
				id: "cc4f608f-c69e-4705-80aa-e69648cfbff2",
				location: [7, { loop: 7, iteration: 3 }, 19],
				kind: {
					type: "step",
					data: {
						output: {
							reservationId:
								"06a7d5bf-b97c-475f-8dea-896cf2b2afa4",
							itemId: "item-4",
						},
					},
				},
				dirty: false,
			},
		},
		{
			key: "short-cooldown",
			entry: {
				id: "cd8929e1-6e36-4fdc-bee6-ef9154e32433",
				location: [20],
				kind: {
					type: "sleep",
					data: { deadline: 1769562508608, state: "completed" },
				},
				dirty: false,
			},
		},
		{
			key: "cooldown-sleep",
			entry: {
				id: "7806b82e-1fda-43f4-8f94-a93559d18f91",
				location: [21],
				kind: {
					type: "sleep",
					data: { deadline: 1769562508870, state: "completed" },
				},
				dirty: false,
			},
		},
		{
			key: "wait-until-deadline",
			entry: {
				id: "b33ae1de-c8ae-48d5-9119-5b78098ea889",
				location: [22],
				kind: {
					type: "sleep",
					data: { deadline: 1769562508725, state: "completed" },
				},
				dirty: false,
			},
		},
		{
			key: "compute-deadlines",
			entry: {
				id: "a806679f-562e-4997-a3ac-cc1f70e3a169",
				location: [23],
				kind: {
					type: "step",
					data: {
						output: {
							readyBy: 1769562509584,
							readyBatchBy: 1769562509884,
						},
					},
				},
				dirty: false,
			},
		},
		{
			key: "listen-order-created:count",
			entry: {
				id: "f6190460-4c9f-45a7-b1e0-9906f1a20151",
				location: [24],
				kind: {
					type: "message",
					data: { name: "order:created:count", data: 1 },
				},
				dirty: false,
			},
		},
		{
			key: "listen-order-created:0",
			entry: {
				id: "f3bb8d48-6d5c-467f-80a3-bd6cc7bcb04e",
				location: [25],
				kind: {
					type: "message",
					data: {
						name: "order:created",
						data: { id: "order-1" },
					},
				},
				dirty: false,
			},
		},
		{
			key: "listen-order-updated-timeout",
			entry: {
				id: "c56ba955-e3d3-4efc-8022-d81aa6cad57b",
				location: [26],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509518, state: "interrupted" },
				},
				dirty: false,
			},
		},
		{
			key: "listen-order-updated-timeout:message",
			entry: {
				id: "428d47e2-96b0-4b49-972f-10264db07107",
				location: [27],
				kind: {
					type: "message",
					data: {
						name: "order:updated",
						data: { id: "order-1", status: "paid" },
					},
				},
				dirty: false,
			},
		},
		{
			key: "listen-batch-two:count",
			entry: {
				id: "e00eb9ae-8d9e-4235-9e42-11e6d843b7a3",
				location: [28],
				kind: {
					type: "message",
					data: { name: "order:item:count", data: 2 },
				},
				dirty: false,
			},
		},
		{
			key: "listen-batch-two:0",
			entry: {
				id: "28bdd95f-18a5-4251-a673-1b17acdac095",
				location: [29],
				kind: {
					type: "message",
					data: {
						name: "order:item",
						data: { sku: "sku-0", qty: 1 },
					},
				},
				dirty: false,
			},
		},
		{
			key: "listen-batch-two:1",
			entry: {
				id: "79c84965-e586-4358-bae4-056f9b6df0ae",
				location: [30],
				kind: {
					type: "message",
					data: {
						name: "order:item",
						data: { sku: "sku-4", qty: 1 },
					},
				},
				dirty: false,
			},
		},
		{
			key: "listen-artifacts-timeout:deadline",
			entry: {
				id: "391ab747-5858-4dd1-b8cd-351342bc8bb6",
				location: [31],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509575, state: "pending" },
				},
				dirty: false,
			},
		},
		{
			key: "listen-artifacts-timeout:count",
			entry: {
				id: "81d3ee26-24c6-4a76-acf0-73a83494e935",
				location: [32],
				kind: {
					type: "message",
					data: { name: "order:artifact:count", data: 3 },
				},
				dirty: false,
			},
		},
		{
			key: "listen-artifacts-timeout:0",
			entry: {
				id: "4f7c0df6-2ea5-4a35-8615-de0df77bc404",
				location: [33],
				kind: {
					type: "message",
					data: {
						name: "order:artifact",
						data: { artifactId: "artifact-0" },
					},
				},
				dirty: false,
			},
		},
		{
			key: "listen-artifacts-timeout:1",
			entry: {
				id: "756ce6e9-d9c6-4842-b87a-a8e91e571062",
				location: [34],
				kind: {
					type: "message",
					data: {
						name: "order:artifact",
						data: { artifactId: "artifact-1" },
					},
				},
				dirty: false,
			},
		},
		{
			key: "listen-artifacts-timeout:2",
			entry: {
				id: "a9dfeafd-1146-47c1-b839-3c41ee12222b",
				location: [35],
				kind: {
					type: "message",
					data: {
						name: "order:artifact",
						data: { artifactId: "artifact-2" },
					},
				},
				dirty: false,
			},
		},
		{
			key: "listen-optional",
			entry: {
				id: "fbff37b3-db46-44f8-9241-87356c41bd01",
				location: [36],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509280, state: "completed" },
				},
				dirty: false,
			},
		},
		{
			key: "listen-until",
			entry: {
				id: "7da0a38f-f5a0-43ab-b299-88ff7e9560ed",
				location: [38],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509584, state: "interrupted" },
				},
				dirty: false,
			},
		},
		{
			key: "listen-until:message",
			entry: {
				id: "dfde8550-8456-4b02-a4a3-bbe767876eb1",
				location: [39],
				kind: {
					type: "message",
					data: {
						name: "order:ready",
						data: { batch: 3 },
					},
				},
				dirty: false,
			},
		},
		{
			key: "listen-batch-until:count",
			entry: {
				id: "66acb760-0d1d-40a6-af52-26d8fa10b3bb",
				location: [40],
				kind: {
					type: "message",
					data: { name: "order:ready:count", data: 2 },
				},
				dirty: false,
			},
		},
		{
			key: "listen-batch-until:0",
			entry: {
				id: "b70c195a-80f9-4d73-9067-e6a347c03165",
				location: [41],
				kind: {
					type: "message",
					data: {
						name: "order:ready",
						data: { batch: 0 },
					},
				},
				dirty: false,
			},
		},
		{
			key: "listen-batch-until:1",
			entry: {
				id: "1f5cbb5b-0df1-432d-9a6f-5ff30eb51c3a",
				location: [42],
				kind: {
					type: "message",
					data: {
						name: "order:ready",
						data: { batch: 2 },
					},
				},
				dirty: false,
			},
		},
		{
			key: "join-dependencies",
			entry: {
				id: "ea93a137-db66-48fb-b1fd-9e6f5d5a084c",
				location: [43],
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
								output: {
									method: "ground",
									etaDays: 4,
									zone: "us-east",
								},
							},
						},
					},
				},
				dirty: false,
			},
		},
		{
			key: "join-dependencies/inventory/inventory-audit",
			entry: {
				id: "7ac02dbe-5dc0-45cb-8c91-bbb102962a1a",
				location: [43, 44, 45],
				kind: {
					type: "step",
					data: { output: 4 },
				},
				dirty: false,
			},
		},
		{
			key: "join-dependencies/inventory/join-inventory-sleep",
			entry: {
				id: "9390eac8-5a96-47a9-8e9c-bc2618c1e5f9",
				location: [43, 44, 50],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509509, state: "completed" },
				},
				dirty: false,
			},
		},
		{
			key: "join-dependencies/pricing/pricing-method",
			entry: {
				id: "7a6047fc-4fd6-4945-89a5-95735b5609d6",
				location: [43, 46, 47],
				kind: {
					type: "step",
					data: { output: "promo" },
				},
				dirty: false,
			},
		},
		{
			key: "join-dependencies/shipping/shipping-zone",
			entry: {
				id: "0603cbf7-bc00-4ad1-8477-1558d6924e15",
				location: [43, 48, 49],
				kind: {
					type: "step",
					data: { output: "us-east" },
				},
				dirty: false,
			},
		},
		{
			key: "join-dependencies/shipping/join-shipping-sleep",
			entry: {
				id: "8de4fa8f-77d5-41c3-b73a-00ad4c66548b",
				location: [43, 48, 51],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509509, state: "completed" },
				},
				dirty: false,
			},
		},
		{
			key: "race-fulfillment",
			entry: {
				id: "623243da-8b14-4b3b-9a83-aa4bdd1a4e43",
				location: [52],
				kind: {
					type: "race",
					data: {
						winner: "race-fast",
						branches: {
							"race-fast": {
								status: "completed",
								output: {
									method: "express",
									cost: 18,
									etaDays: 1,
								},
							},
							"race-slow": {
								status: "failed",
								error: "SleepError: Sleeping until 1769562509793",
							},
						},
					},
				},
				dirty: false,
			},
		},
		{
			key: "race-fulfillment/race-fast/race-fast-sleep",
			entry: {
				id: "74dc4690-7567-4dee-9067-33d7d9d8b9ef",
				location: [52, 53, 54],
				kind: {
					type: "sleep",
					data: { deadline: 1769562509643, state: "completed" },
				},
				dirty: false,
			},
		},
		{
			key: "legacy-step-placeholder",
			entry: {
				id: "183dae7e-5650-481d-bf74-fe6af534ab9e",
				location: [57],
				kind: {
					type: "removed",
					data: {
						originalType: "step",
						originalName: "legacy-step-placeholder",
					},
				},
				dirty: false,
			},
		},
		{
			key: "finalize",
			entry: {
				id: "d9da246b-54c7-4681-afef-3eda5c2f66c0",
				location: [58],
				kind: {
					type: "step",
					data: { output: true },
				},
				dirty: false,
			},
		},
	],
};

// Workflow with in-progress step
export const inProgressWorkflow: WorkflowHistory = {
	workflowId: "in-progress-001",
	state: "running",
	nameRegistry: ["init", "fetch-data", "process"],
	history: [
		{
			key: "init",
			entry: {
				id: "1",
				location: [0],
				kind: {
					type: "step",
					data: { output: { initialized: true } },
				},
				dirty: false,
				status: "completed",
				startedAt: 1700000400000,
				completedAt: 1700000400050,
			},
		},
		{
			key: "fetch-data",
			entry: {
				id: "2",
				location: [1],
				kind: {
					type: "step",
					data: { output: { fetched: true, records: 100 } },
				},
				dirty: false,
				status: "completed",
				startedAt: 1700000400060,
				completedAt: 1700000400350,
			},
		},
		{
			key: "process",
			entry: {
				id: "3",
				location: [2],
				kind: {
					type: "step",
					data: { output: { processing: "batch-1", progress: 42 } },
				},
				dirty: true,
				status: "running",
				startedAt: 1700000400360,
			},
		},
	],
};

// Workflow with retrying step
export const retryWorkflow: WorkflowHistory = {
	workflowId: "retry-workflow-001",
	state: "running",
	nameRegistry: ["start", "api-call"],
	history: [
		{
			key: "start",
			entry: {
				id: "1",
				location: [0],
				kind: {
					type: "step",
					data: { output: { ready: true } },
				},
				dirty: false,
				status: "completed",
				startedAt: 1700000500000,
				completedAt: 1700000500040,
			},
		},
		{
			key: "api-call",
			entry: {
				id: "2",
				location: [1],
				kind: { type: "step", data: { output: { attempt: 3 } } },
				dirty: true,
				status: "retrying",
				startedAt: 1700000500050,
				retryCount: 2,
				error: "Connection timeout after 5000ms",
			},
		},
	],
};

// Workflow with failed step
export const failedWorkflow: WorkflowHistory = {
	workflowId: "failed-workflow-001",
	state: "failed",
	nameRegistry: ["init", "validate", "process"],
	history: [
		{
			key: "init",
			entry: {
				id: "1",
				location: [0],
				kind: {
					type: "step",
					data: { output: { initialized: true } },
				},
				dirty: false,
				status: "completed",
				startedAt: 1700000600000,
				completedAt: 1700000600030,
			},
		},
		{
			key: "validate",
			entry: {
				id: "2",
				location: [1],
				kind: {
					type: "step",
					data: { output: { valid: true } },
				},
				dirty: false,
				status: "completed",
				startedAt: 1700000600040,
				completedAt: 1700000600120,
			},
		},
		{
			key: "process",
			entry: {
				id: "3",
				location: [2],
				kind: { type: "step", data: {} },
				dirty: false,
				status: "failed",
				startedAt: 1700000600130,
				completedAt: 1700000600280,
				retryCount: 3,
				error: "Database connection failed: ECONNREFUSED",
			},
		},
	],
};

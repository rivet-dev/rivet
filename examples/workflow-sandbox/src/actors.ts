// Workflow Sandbox - Actor Registry
// Each actor demonstrates a different workflow feature using actor-per-workflow pattern

import { setup } from "rivetkit";

// Import actors from individual files
export { timer } from "./actors/timer.ts";
export type { Timer, TimerInput } from "./actors/timer.ts";

export { order } from "./actors/order.ts";
export type { Order, OrderStatus } from "./actors/order.ts";

export { batch } from "./actors/batch.ts";
export type { BatchInfo, BatchJob, BatchJobInput } from "./actors/batch.ts";

export { approval } from "./actors/approval.ts";
export type {
	ApprovalRequest,
	ApprovalRequestInput,
	RequestStatus,
} from "./actors/approval.ts";

export { dashboard } from "./actors/dashboard.ts";
export type {
	DashboardData,
	DashboardState,
	UserStats,
	OrderStats,
	MetricsStats,
	BranchStatus,
} from "./actors/dashboard.ts";

export { race } from "./actors/race.ts";
export type { RaceTask, RaceTaskInput } from "./actors/race.ts";

export { payment } from "./actors/payment.ts";
export type {
	Transaction,
	TransactionStep,
	TransactionInput,
} from "./actors/payment.ts";

// Import for registry setup
import { timer } from "./actors/timer.ts";
import { order } from "./actors/order.ts";
import { batch } from "./actors/batch.ts";
import { approval } from "./actors/approval.ts";
import { dashboard } from "./actors/dashboard.ts";
import { race } from "./actors/race.ts";
import { payment } from "./actors/payment.ts";

// Registry setup
export const registry = setup({
	use: {
		timer,
		order,
		batch,
		approval,
		dashboard,
		race,
		payment,
	},
});

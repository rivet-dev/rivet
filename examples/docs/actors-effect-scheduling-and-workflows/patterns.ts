import { Workflow } from "@rivetkit/effect";
import { Effect } from "effect";

// Stand-ins for effects that call other systems.
const isExportReady = Effect.succeed(true);
const reserveSeat = Effect.succeed("12A");
const releaseSeat = (seat: string) => Effect.log(`Released seat ${seat}`);
const chargeCard = (seat: string) => Effect.log(`Charged for seat ${seat}`);
const sendEmail = Effect.log("Email sent");
const sendSms = Effect.log("SMS sent");
const quoteAir = Effect.succeed(120);
const quoteSea = Effect.succeed(80);
const waitForApproval = Effect.succeed("approved");
const checkFraud = Effect.log("Fraud check passed");

// docs:start loop
export const pollWorkflow = Workflow.loop(
	"poll",
	Effect.gen(function* () {
		const ready = yield* Workflow.step("check", isExportReady);
		if (ready) return Workflow.loopBreak;
		yield* Workflow.sleep("wait", "1 minute");
	}),
);
// docs:end loop

// docs:start rollback
export const bookingWorkflow = Effect.gen(function* () {
	yield* Workflow.rollbackCheckpoint("booking");
	const seat = yield* Workflow.step("reserve-seat", reserveSeat, {
		rollback: (seat) => releaseSeat(seat),
	});

	// If this step fails after its retries, RivetKit runs `releaseSeat`.
	yield* Workflow.step("charge", chargeCard(seat));
});
// docs:end rollback

// docs:start try
export const notifyWorkflow = Effect.gen(function* () {
	const email = yield* Workflow.tryStep("send-email", sendEmail).pipe(
		Effect.as("sent"),
		Effect.catchTag("WorkflowStepFailure", (failure) =>
			Effect.succeed(`not sent: ${failure.message}`),
		),
	);
	yield* Effect.log(`Email ${email}`);

	yield* Workflow.try(
		"fallbacks",
		Effect.gen(function* () {
			yield* Workflow.step("send-sms", sendSms);
			yield* Workflow.step("send-second-email", sendEmail);
		}),
	).pipe(Effect.catchTag("WorkflowTryFailure", () => Effect.void));
});
// docs:end try

// docs:start concurrent
export const shippingWorkflow = Effect.gen(function* () {
	const quotes = yield* Workflow.join("quotes", {
		air: Workflow.step("quote", quoteAir),
		sea: Workflow.step("quote", quoteSea),
	});
	yield* Effect.log(`Best quote: ${Math.min(quotes.air, quotes.sea)}`);

	const approval = yield* Workflow.race("approval", {
		approved: Workflow.step("wait-for-approval", waitForApproval),
		expired: Workflow.sleep("approval-deadline", "1 day"),
	});
	if (approval.winner === "expired") return;
	yield* Effect.log(`Approval: ${approval.value}`);
});
// docs:end concurrent

// docs:start versioning
export const versionedWorkflow = Effect.gen(function* () {
	// A new workflow gets 2. A workflow that ran past this point before gets 1.
	const version = yield* Workflow.getVersion("fraud-check", 2);
	if (version >= 2) {
		yield* Workflow.step("check-fraud", checkFraud);
	}

	// Stands in for the deleted step "send-fax".
	yield* Workflow.removed("send-fax", "step");
});
// docs:end versioning

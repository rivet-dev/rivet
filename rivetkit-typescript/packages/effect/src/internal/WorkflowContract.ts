/**
 * `Workflow.make` lives outside `Workflow.ts` so that a contract does not
 * import server code.
 */

import { makeContract } from "../Actor.ts";

/**
 * Define a Rivet workflow contract: an actor contract whose `toLayer` takes
 * the durable workflow body. It is the Effect form of
 * `workflow({ run, actions })` from `@rivet-dev/workflows`: the first
 * argument of `toLayer` gives the workflow body, and `actions` in its options
 * gives the action handlers.
 *
 * The first argument of `toLayer` is an effect that succeeds with the
 * workflow body. It runs on each wake in the wake scope, with the services of
 * an actor's wake effect, so the body can use what it builds. The `actions`
 * effect runs after it. The body starts only after both completed, and a
 * failure of either fails the wake.
 *
 * RivetKit runs the body in the background, again from the start on every
 * wake. Finished `Workflow.step`, `Workflow.sleep` and `Workflow.loop` calls
 * return their saved results, so everything outside a step must be
 * deterministic. The body runs with the services of an action handler and
 * `Workflow.Workflow`. The actor does not idle-sleep while the body runs or
 * waits in `Workflow.sleep`. When the body returns, the workflow is complete
 * and the actor sleeps once it is idle. A defect outside a step fails the
 * workflow.
 *
 * If RivetKit starts the workflow when the wake did not complete, or after
 * the actor's teardown, the body does not start and nothing is saved. The
 * next wake starts it.
 *
 * @example
 * ```ts
 * export const Order = Workflow.make("Order", { actions: [GetStatus] });
 *
 * export const OrderLive = Order.toLayer(
 *   Effect.gen(function* () {
 *     const payments = yield* Payments;
 *
 *     return Effect.gen(function* () {
 *       yield* Workflow.step("charge", payments.charge);
 *       yield* Workflow.sleep("wait-for-shipping", "1 hour");
 *       yield* Workflow.step("ship", payments.ship);
 *     });
 *   }),
 *   {
 *     actions: Effect.gen(function* () {
 *       const payments = yield* Payments;
 *
 *       return Order.of({ GetStatus: () => payments.status });
 *     }),
 *   },
 * );
 * ```
 */
export const make = makeContract("workflow");

import { setup } from "rivetkit";
// This file is the single static registry source for the driver fixtures.
// Static runs import this registry directly, and dynamic runs reuse its types and actor metadata.
import {
	accessControlActor,
	accessControlNoQueuesActor,
} from "./access-control";

import { inputActor } from "./action-inputs";
import {
	defaultTimeoutActor,
	longTimeoutActor,
	shortTimeoutActor,
	syncTimeoutActor,
} from "./action-timeout";
import {
	asyncActionActor,
	concurrentActionActor,
	promiseActor,
	syncActionActor,
} from "./action-types";
import { dbActorRaw } from "./actor-db-raw";
import { onStateChangeActor } from "./actor-onstatechange";
import { connErrorSerializationActor } from "./conn-error-serialization";
import { dbPragmaMigrationActor } from "./db-pragma-migration";
import { counterWithParams } from "./conn-params";
import { connStateActor } from "./conn-state";
// Import actors from individual files
import { counter } from "./counter";
import { counterConn } from "./counter-conn";
import {
	dbLifecycle,
	dbLifecycleFailing,
	dbLifecycleObserver,
} from "./db-lifecycle";
import { destroyActor, destroyObserver } from "./destroy";
import { customTimeoutActor, errorHandlingActor } from "./error-handling";
import { fileSystemHibernationCleanupActor } from "./file-system-hibernation-cleanup";
import { hibernationActor, hibernationSleepWindowActor } from "./hibernation";
import {
	beforeConnectTimeoutActor,
	beforeConnectRejectActor,
	beforeConnectGenericErrorActor,
	stateChangeRecursionActor,
	stateChangeReentrantMutationActor,
} from "./lifecycle-hooks";
import { kvActor } from "./kv";
import { largePayloadActor, largePayloadConnActor } from "./large-payloads";
import { counterWithLifecycle } from "./lifecycle";
import { metadataActor } from "./metadata";
import {
	manyQueueActionParentActor,
	manyQueueChildActor,
	manyQueueRunParentActor,
	queueActor,
	queueLimitedActor,
} from "./queue";
import {
	rawHttpActor,
	rawHttpHonoActor,
	rawHttpNoHandlerActor,
	rawHttpVoidReturnActor,
} from "./raw-http";
import { rawHttpRequestPropertiesActor } from "./raw-http-request-properties";
import { rawWebSocketActor, rawWebSocketBinaryActor } from "./raw-websocket";
import { rejectConnectionActor } from "./reject-connection";
import { requestAccessActor } from "./request-access";
import {
	runSelfInitiatedDestroy,
	runSelfInitiatedSleep,
	runIgnoresAbortStopTimeout,
	runWithEarlyExit,
	runWithError,
	runWithoutHandler,
	runWithQueueConsumer,
	runWithTicks,
} from "./run";
import { scheduled } from "./scheduled";
import { dbStressActor } from "./db-stress";
import { scheduledDb } from "./scheduled-db";
import {
	sleep,
	sleepRawWsAddEventListenerClose,
	sleepRawWsAddEventListenerMessage,
	sleepWithLongRpc,
	sleepWithNoSleepOption,
	sleepWithPreventSleep,
	sleepWithRawHttp,
	sleepWithRawWebSocket,
	sleepWithWaitUntilMessage,
	sleepRawWsOnClose,
	sleepRawWsOnMessage,
	sleepRawWsSendOnSleep,
	sleepRawWsDelayedSendOnSleep,
	sleepWithWaitUntilInOnWake,
} from "./sleep";
import {
	sleepWithDb,
	sleepWithSlowScheduledDb,
	sleepWithDbConn,
	sleepWithDbAction,
	sleepWithRawWsCloseDb,
	sleepWithRawWsCloseDbListener,
	sleepWsMessageExceedsGrace,
	sleepWsConcurrentDbExceedsGrace,
	sleepWaitUntil,
	sleepNestedWaitUntil,
	sleepEnqueue,
	sleepScheduleAfter,
	sleepOnSleepThrows,
	sleepWaitUntilRejects,
	sleepWaitUntilState,
	sleepWithRawWs,
	sleepWsActiveDbExceedsGrace,
} from "./sleep-db";
import { lifecycleObserver, startStopRaceActor } from "./start-stop-race";
import { statelessActor } from "./stateless";
import { stateZodCoercionActor } from "./state-zod-coercion";
import {
	driverCtxActor,
	dynamicVarActor,
	nestedVarActor,
	staticVarActor,
	uniqueVarActor,
} from "./vars";
import {
	workflowAccessActor,
	workflowCompleteActor,
	workflowCounterActor,
	workflowDestroyActor,
	workflowErrorHookActor,
	workflowErrorHookEffectsActor,
	workflowErrorHookSleepActor,
	workflowFailedStepActor,
	workflowNestedJoinActor,
	workflowNestedLoopActor,
	workflowNestedRaceActor,
	workflowQueueActor,
	workflowRunningStepActor,
	workflowReplayActor,
	workflowSleepActor,
	workflowSpawnChildActor,
	workflowSpawnParentActor,
	workflowStopTeardownActor,
	workflowTryActor,
} from "./workflow";

let agentOsTestActor:
	| Awaited<typeof import("./agent-os")>["agentOsTestActor"]
	| undefined;

try {
	({ agentOsTestActor } = await import("./agent-os"));
} catch (error) {
	if (!(error instanceof Error) || !error.message.includes("agent-os")) {
		throw error;
	}
}

// Consolidated setup with all actors
export const registry = setup({
	use: {
		// From counter.ts
		counter,
		// From counter-conn.ts
		counterConn,
		// From lifecycle.ts
		counterWithLifecycle,
		// From scheduled.ts
		scheduled,
		// From db-stress.ts
		dbStressActor,
		// From scheduled-db.ts
		scheduledDb,
		// From sleep.ts
		sleep,
		sleepWithLongRpc,
		sleepWithRawHttp,
		sleepWithRawWebSocket,
		sleepWithNoSleepOption,
		sleepWithPreventSleep,
		sleepWithWaitUntilMessage,
		sleepRawWsAddEventListenerMessage,
		sleepRawWsAddEventListenerClose,
		sleepRawWsOnMessage,
		sleepRawWsOnClose,
		sleepRawWsSendOnSleep,
		sleepRawWsDelayedSendOnSleep,
		sleepWithWaitUntilInOnWake,
		// From sleep-db.ts
		sleepWithDb,
		sleepWithSlowScheduledDb,
		sleepWithDbConn,
		sleepWithDbAction,
		sleepWaitUntil,
		sleepNestedWaitUntil,
		sleepEnqueue,
		sleepScheduleAfter,
		sleepOnSleepThrows,
		sleepWaitUntilRejects,
		sleepWaitUntilState,
		sleepWithRawWs,
		sleepWithRawWsCloseDb,
		sleepWithRawWsCloseDbListener,
		sleepWsMessageExceedsGrace,
		sleepWsConcurrentDbExceedsGrace,
		sleepWsActiveDbExceedsGrace,
		// From error-handling.ts
		errorHandlingActor,
		customTimeoutActor,
		// From kv.ts
		kvActor,
		// From queue.ts
		queueActor,
		queueLimitedActor,
		manyQueueChildActor,
		manyQueueActionParentActor,
		manyQueueRunParentActor,
		// From action-inputs.ts
		inputActor,
		// From action-timeout.ts
		shortTimeoutActor,
		longTimeoutActor,
		defaultTimeoutActor,
		syncTimeoutActor,
		// From action-types.ts
		syncActionActor,
		asyncActionActor,
		concurrentActionActor,
		promiseActor,
		// From conn-params.ts
		counterWithParams,
		// From conn-state.ts
		connStateActor,
		// From metadata.ts
		metadataActor,
		// From vars.ts
		staticVarActor,
		nestedVarActor,
		dynamicVarActor,
		uniqueVarActor,
		driverCtxActor,
		// From raw-http.ts
		rawHttpActor,
		rawHttpNoHandlerActor,
		rawHttpVoidReturnActor,
		rawHttpHonoActor,
		// From raw-http-request-properties.ts
		rawHttpRequestPropertiesActor,
		// From raw-websocket.ts
		rawWebSocketActor,
		rawWebSocketBinaryActor,
		// From reject-connection.ts
		rejectConnectionActor,
		// From request-access.ts
		requestAccessActor,
		// From actor-onstatechange.ts
		onStateChangeActor,
		// From destroy.ts
		destroyActor,
		destroyObserver,
		// From hibernation.ts
		hibernationActor,
		hibernationSleepWindowActor,
		// From file-system-hibernation-cleanup.ts
		fileSystemHibernationCleanupActor,
		// From large-payloads.ts
		largePayloadActor,
		largePayloadConnActor,
		// From run.ts
		runWithTicks,
		runWithQueueConsumer,
		runWithEarlyExit,
		runWithError,
		runSelfInitiatedSleep,
		runSelfInitiatedDestroy,
		runIgnoresAbortStopTimeout,
		runWithoutHandler,
		// From workflow.ts
		workflowCounterActor,
		workflowQueueActor,
		workflowAccessActor,
		workflowCompleteActor,
		workflowDestroyActor,
		workflowFailedStepActor,
		workflowRunningStepActor,
		workflowReplayActor,
		workflowSleepActor,
		workflowTryActor,
		workflowStopTeardownActor,
		workflowErrorHookActor,
		workflowErrorHookEffectsActor,
		workflowErrorHookSleepActor,
		workflowNestedLoopActor,
		workflowNestedJoinActor,
		workflowNestedRaceActor,
		workflowSpawnChildActor,
		workflowSpawnParentActor,
		// From actor-db-raw.ts
		dbActorRaw,
		// From db-lifecycle.ts
		dbLifecycle,
		dbLifecycleFailing,
		dbLifecycleObserver,
		// From stateless.ts
		statelessActor,
		// From access-control.ts
		accessControlActor,
		accessControlNoQueuesActor,
		// From start-stop-race.ts
		startStopRaceActor,
		lifecycleObserver,
		// From conn-error-serialization.ts
		connErrorSerializationActor,
		// From db-pragma-migration.ts
		dbPragmaMigrationActor,
		// From state-zod-coercion.ts
		stateZodCoercionActor,
		// From lifecycle-hooks.ts
		beforeConnectTimeoutActor,
		beforeConnectRejectActor,
		beforeConnectGenericErrorActor,
		stateChangeRecursionActor,
		stateChangeReentrantMutationActor,
		...(agentOsTestActor
			? {
					// From agent-os.ts
					agentOsTestActor,
				}
			: {}),
	},
});

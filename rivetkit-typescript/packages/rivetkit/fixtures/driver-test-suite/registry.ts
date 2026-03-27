import { setup } from "rivetkit";
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
	promiseActor,
	syncActionActor,
} from "./action-types";
import { dbActorDrizzle } from "./actor-db-drizzle";
import { dbActorRaw } from "./actor-db-raw";
import { onStateChangeActor } from "./actor-onstatechange";
import { connErrorSerializationActor } from "./conn-error-serialization";
import { dbPragmaMigrationActor } from "./db-pragma-migration";
import { counterWithParams } from "./conn-params";
import { connStateActor } from "./conn-state";
// Import actors from individual files
import { counter } from "./counter";
import { counterConn } from "./counter-conn";
import { dbKvStatsActor } from "./db-kv-stats";
import {
	dbLifecycle,
	dbLifecycleFailing,
	dbLifecycleObserver,
} from "./db-lifecycle";
import { destroyActor, destroyObserver } from "./destroy";
import { customTimeoutActor, errorHandlingActor } from "./error-handling";
import { fileSystemHibernationCleanupActor } from "./file-system-hibernation-cleanup";
import {
	hibernationActor,
	hibernationSleepWindowActor,
} from "./hibernation";
import { inlineClientActor } from "./inline-client";
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
	runWithEarlyExit,
	runWithError,
	runWithoutHandler,
	runWithQueueConsumer,
	runWithTicks,
} from "./run";
import { dockerSandboxActor } from "./sandbox";
import { scheduled } from "./scheduled";
import { scheduledDb } from "./scheduled-db";
import {
	sleep,
	sleepWithLongRpc,
	sleepWithNoSleepOption,
	sleepWithPreventSleep,
	sleepWithRawHttp,
	sleepWithRawWebSocket,
	sleepWithWaitUntilMessage,
	sleepRawWsSendOnSleep,
	sleepRawWsDelayedSendOnSleep,
} from "./sleep";
import {
	sleepWithDb,
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
} from "./workflow";

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
		// From scheduled-db.ts
		scheduledDb,
		// From sandbox.ts
		dockerSandboxActor,
		// From sleep.ts
		sleep,
		sleepWithLongRpc,
		sleepWithRawHttp,
		sleepWithRawWebSocket,
		sleepWithNoSleepOption,
		sleepWithPreventSleep,
		sleepWithWaitUntilMessage,
		sleepRawWsSendOnSleep,
		sleepRawWsDelayedSendOnSleep,
		// From sleep-db.ts
		sleepWithDb,
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
		// From error-handling.ts
		errorHandlingActor,
		customTimeoutActor,
		// From inline-client.ts
		inlineClientActor,
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
		// From actor-db-drizzle.ts
		dbActorDrizzle,
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
		// From db-kv-stats.ts
		dbKvStatsActor,
		// From db-pragma-migration.ts
		dbPragmaMigrationActor,
		// From state-zod-coercion.ts
		stateZodCoercionActor,
	},
});

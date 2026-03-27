import { setup } from "rivetkit";
// Actions
import { inputActor } from "./actors/actions/action-inputs.ts";
import {
	defaultTimeoutActor,
	longTimeoutActor,
	shortTimeoutActor,
	syncTimeoutActor,
} from "./actors/actions/action-timeout.ts";
import {
	asyncActionActor,
	promiseActor,
	syncActionActor,
} from "./actors/actions/action-types.ts";
import {
	customTimeoutActor,
	errorHandlingActor,
} from "./actors/actions/error-handling.ts";
// AI
import { aiAgent } from "./actors/ai/ai-agent.ts";
// Connections
import { connStateActor } from "./actors/connections/conn-state.ts";
import { rejectConnectionActor } from "./actors/connections/reject-connection.ts";
import { requestAccessActor } from "./actors/connections/request-access.ts";
import { counterWithParams } from "./actors/counter/conn-params.ts";
// Counter
import { counter } from "./actors/counter/counter.ts";
import { counterConn } from "./actors/counter/counter-conn.ts";
import { counterWithLifecycle } from "./actors/counter/lifecycle.ts";
import { rawFetchCounter } from "./actors/http/raw-fetch-counter.ts";
// HTTP
import {
	rawHttpActor,
	rawHttpHonoActor,
	rawHttpNoHandlerActor,
	rawHttpVoidReturnActor,
} from "./actors/http/raw-http.ts";
import { rawHttpRequestPropertiesActor } from "./actors/http/raw-http-request-properties.ts";
import {
	rawWebSocketActor,
	rawWebSocketBinaryActor,
} from "./actors/http/raw-websocket.ts";
import { rawWebSocketChatRoom } from "./actors/http/raw-websocket-chat-room.ts";
// Inter-actor
import {
	checkout,
	inventory,
} from "./actors/inter-actor/cross-actor-actions.ts";
import { destroyActor, destroyObserver } from "./actors/lifecycle/destroy.ts";
import { hibernationActor } from "./actors/lifecycle/hibernation.ts";
// Lifecycle
import {
	runWithEarlyExit,
	runWithError,
	runWithoutHandler,
	runWithQueueConsumer,
	runWithTicks,
} from "./actors/lifecycle/run.ts";
import { scheduled } from "./actors/lifecycle/scheduled.ts";
import {
	sleep,
	sleepWithLongRpc,
	sleepWithNoSleepOption,
	sleepWithRawHttp,
	sleepWithRawWebSocket,
} from "./actors/lifecycle/sleep.ts";
// Queues
import { worker } from "./actors/queue/worker.ts";
import { workerTimeout } from "./actors/queue/worker-timeout.ts";
// State
import { onStateChangeActor } from "./actors/state/actor-onstatechange.ts";
import { kvActor } from "./actors/state/kv.ts";
import {
	largePayloadActor,
	largePayloadConnActor,
} from "./actors/state/large-payloads.ts";
import { metadataActor } from "./actors/state/metadata.ts";
import { parallelismTest } from "./actors/state/parallelism-test.ts";
import { sqliteDrizzleActor } from "./actors/state/sqlite-drizzle/mod.ts";
import { sqliteRawActor } from "./actors/state/sqlite-raw.ts";
import {
	driverCtxActor,
	dynamicVarActor,
	nestedVarActor,
	staticVarActor,
	uniqueVarActor,
} from "./actors/state/vars.ts";
// Testing
import { inlineClientActor } from "./actors/testing/inline-client.ts";
import { testCounter } from "./actors/testing/test-counter.ts";
import { testCounterSqlite } from "./actors/testing/test-counter-sqlite.ts";
import { testSqliteLoad } from "./actors/testing/test-sqlite-load.ts";
import { approval } from "./actors/workflow/approval.ts";
import { batch } from "./actors/workflow/batch.ts";
import { dashboard } from "./actors/workflow/dashboard.ts";
import {
	workflowHistoryFailed,
	workflowHistoryFull,
	workflowHistoryInProgress,
	workflowHistoryJoin,
	workflowHistoryLoop,
	workflowHistoryRace,
	workflowHistoryRetrying,
	workflowHistorySimple,
} from "./actors/workflow/history-examples.ts";
import { order } from "./actors/workflow/order.ts";
import { payment } from "./actors/workflow/payment.ts";
import { race } from "./actors/workflow/race.ts";
import { timer } from "./actors/workflow/timer.ts";
// Workflows
import {
	workflowCounterActor,
	workflowQueueActor,
	workflowQueueTimeoutActor,
	workflowSleepActor,
} from "./actors/workflow/workflow-fixtures.ts";

export const registry = setup({
	use: {
		// Overview + state basics
		counter,
		counterConn,
		counterWithParams,
		counterWithLifecycle,
		// Core API
		inputActor,
		syncActionActor,
		asyncActionActor,
		promiseActor,
		shortTimeoutActor,
		longTimeoutActor,
		defaultTimeoutActor,
		syncTimeoutActor,
		customTimeoutActor,
		errorHandlingActor,
		// State and storage
		onStateChangeActor,
		metadataActor,
		staticVarActor,
		nestedVarActor,
		dynamicVarActor,
		uniqueVarActor,
		driverCtxActor,
		kvActor,
		largePayloadActor,
		largePayloadConnActor,
		sqliteRawActor,
		sqliteDrizzleActor,
		parallelismTest,
		// Realtime and connections
		connStateActor,
		rejectConnectionActor,
		requestAccessActor,
		// HTTP and WebSocket
		rawHttpActor,
		rawHttpNoHandlerActor,
		rawHttpVoidReturnActor,
		rawHttpHonoActor,
		rawHttpRequestPropertiesActor,
		rawWebSocketActor,
		rawWebSocketBinaryActor,
		rawFetchCounter,
		rawWebSocketChatRoom,
		// Lifecycle and scheduling
		runWithTicks,
		runWithQueueConsumer,
		runWithEarlyExit,
		runWithError,
		runWithoutHandler,
		sleep,
		sleepWithLongRpc,
		sleepWithNoSleepOption,
		sleepWithRawHttp,
		sleepWithRawWebSocket,
		scheduled,
		destroyActor,
		destroyObserver,
		hibernationActor,
		// Queues
		worker,
		workerTimeout,
		// Workflows
		timer,
		order,
		batch,
		approval,
		dashboard,
		race,
		payment,
		workflowHistorySimple,
		workflowHistoryLoop,
		workflowHistoryJoin,
		workflowHistoryRace,
		workflowHistoryFull,
		workflowHistoryInProgress,
		workflowHistoryRetrying,
		workflowHistoryFailed,
		workflowCounterActor,
		workflowQueueActor,
		workflowSleepActor,
		workflowQueueTimeoutActor,
		// Inter-actor
		inventory,
		checkout,
		// Testing fixtures
		inlineClientActor,
		testCounter,
		testCounterSqlite,
		testSqliteLoad,
		// AI
		aiAgent,
	},
});

import { setup } from "rivetkit";
// Counter
import { counter } from "./actors/counter/counter.ts";
import { counterConn } from "./actors/counter/counter-conn.ts";
import { counterWithParams } from "./actors/counter/conn-params.ts";
import { counterWithLifecycle } from "./actors/counter/lifecycle.ts";
// Actions
import { inputActor } from "./actors/actions/action-inputs.ts";
import {
	syncActionActor,
	asyncActionActor,
	promiseActor,
} from "./actors/actions/action-types.ts";
import {
	shortTimeoutActor,
	longTimeoutActor,
	defaultTimeoutActor,
	syncTimeoutActor,
} from "./actors/actions/action-timeout.ts";
import {
	errorHandlingActor,
	customTimeoutActor,
} from "./actors/actions/error-handling.ts";
// State
import { onStateChangeActor } from "./actors/state/actor-onstatechange.ts";
import { metadataActor } from "./actors/state/metadata.ts";
import {
	staticVarActor,
	nestedVarActor,
	dynamicVarActor,
	uniqueVarActor,
	driverCtxActor,
} from "./actors/state/vars.ts";
import { kvActor } from "./actors/state/kv.ts";
import {
	largePayloadActor,
	largePayloadConnActor,
} from "./actors/state/large-payloads.ts";
import { sqliteRawActor } from "./actors/state/sqlite-raw.ts";
import { sqliteDrizzleActor } from "./actors/state/sqlite-drizzle/mod.ts";
import { parallelismTest } from "./actors/state/parallelism-test.ts";
// Connections
import { connStateActor } from "./actors/connections/conn-state.ts";
import { rejectConnectionActor } from "./actors/connections/reject-connection.ts";
import { requestAccessActor } from "./actors/connections/request-access.ts";
// HTTP
import {
	rawHttpActor,
	rawHttpNoHandlerActor,
	rawHttpVoidReturnActor,
	rawHttpHonoActor,
} from "./actors/http/raw-http.ts";
import { rawHttpRequestPropertiesActor } from "./actors/http/raw-http-request-properties.ts";
import {
	rawWebSocketActor,
	rawWebSocketBinaryActor,
} from "./actors/http/raw-websocket.ts";
import { rawFetchCounter } from "./actors/http/raw-fetch-counter.ts";
import { rawWebSocketChatRoom } from "./actors/http/raw-websocket-chat-room.ts";
import { rawWebSocketServerlessSmoke } from "./actors/http/raw-websocket-serverless-smoke.ts";
// Lifecycle
import {
	runWithTicks,
	runWithQueueConsumer,
	runWithEarlyExit,
	runWithError,
	runWithoutHandler,
} from "./actors/lifecycle/run.ts";
import {
	sleep,
	sleepWithLongRpc,
	sleepWithNoSleepOption,
	sleepWithRawHttp,
	sleepWithRawWebSocket,
} from "./actors/lifecycle/sleep.ts";
import { scheduled } from "./actors/lifecycle/scheduled.ts";
import {
	destroyActor,
	destroyObserver,
} from "./actors/lifecycle/destroy.ts";
import { hibernationActor } from "./actors/lifecycle/hibernation.ts";
// Queues
import { worker } from "./actors/queue/worker.ts";
import { workerTimeout } from "./actors/queue/worker-timeout.ts";
// Workflows
import {
	workflowCounterActor,
	workflowQueueActor,
	workflowSleepActor,
	workflowQueueTimeoutActor,
} from "./actors/workflow/workflow-fixtures.ts";
import { timer } from "./actors/workflow/timer.ts";
import { order } from "./actors/workflow/order.ts";
import { batch } from "./actors/workflow/batch.ts";
import { approval } from "./actors/workflow/approval.ts";
import { dashboard } from "./actors/workflow/dashboard.ts";
import { race } from "./actors/workflow/race.ts";
import { payment } from "./actors/workflow/payment.ts";
import {
	workflowHistorySimple,
	workflowHistoryLoop,
	workflowHistoryJoin,
	workflowHistoryRace,
	workflowHistoryFull,
	workflowHistoryInProgress,
	workflowHistoryRetrying,
	workflowHistoryFailed,
} from "./actors/workflow/history-examples.ts";
// Inter-actor
import {
	inventory,
	checkout,
} from "./actors/inter-actor/cross-actor-actions.ts";
// Testing
import { inlineClientActor } from "./actors/testing/inline-client.ts";
import { testCounter } from "./actors/testing/test-counter.ts";
import { testCounterSqlite } from "./actors/testing/test-counter-sqlite.ts";
import { testSqliteLoad } from "./actors/testing/test-sqlite-load.ts";
import { testSqliteBench } from "./actors/testing/test-sqlite-bench.ts";
// AI
import { aiAgent } from "./actors/ai/ai-agent.ts";

function numberFromEnv(name: string, fallback: number): number {
	const value = process.env[name];
	if (value === undefined || value === "") return fallback;

	const parsed = Number(value);
	if (!Number.isFinite(parsed)) {
		throw new Error(`${name} must be a finite number`);
	}

	return parsed;
}

function serverlessPoolConfig() {
	const url =
		process.env.RIVET_SERVERLESS_URL ??
		process.env.KITCHEN_SINK_SERVERLESS_URL ??
		(process.env.RIVET_RUN_ENGINE === "1"
			? "http://127.0.0.1:3000/api/rivet"
			: undefined);

	if (!url) return undefined;

	return {
		name: process.env.RIVET_POOL,
		url,
		requestLifespan: numberFromEnv("RIVET_SERVERLESS_REQUEST_LIFESPAN", 30),
		drainGracePeriod: numberFromEnv("RIVET_SERVERLESS_DRAIN_GRACE_PERIOD", 5),
		metadataPollInterval: numberFromEnv(
			"RIVET_SERVERLESS_METADATA_POLL_INTERVAL_MS",
			1000,
		),
		metadata: {
			source: "kitchen-sink-vercel",
			smoke: "raw-websocket-serverless",
		},
	};
}

export const registry = setup({
	configurePool: serverlessPoolConfig(),
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
		rawWebSocketServerlessSmoke,
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
		testSqliteBench,
		// AI
		aiAgent,
	},
});

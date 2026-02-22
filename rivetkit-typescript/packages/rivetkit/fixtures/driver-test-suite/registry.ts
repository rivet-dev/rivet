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
import { dbActorRaw } from "./actor-db-raw";
import { dbActorDrizzle } from "./actor-db-drizzle";
import { onStateChangeActor } from "./actor-onstatechange";
import { counterWithParams } from "./conn-params";
import { connStateActor } from "./conn-state";
// Import actors from individual files
import { counter } from "./counter";
import { counterConn } from "./counter-conn";
import { destroyActor, destroyObserver } from "./destroy";
import { customTimeoutActor, errorHandlingActor } from "./error-handling";
import { fileSystemHibernationCleanupActor } from "./file-system-hibernation-cleanup";
import { hibernationActor } from "./hibernation";
import { inlineClientActor } from "./inline-client";
import { kvActor } from "./kv";
import { largePayloadActor, largePayloadConnActor } from "./large-payloads";
import { counterWithLifecycle } from "./lifecycle";
import { metadataActor } from "./metadata";
import { queueActor, queueLimitedActor } from "./queue";
import {
	rawHttpActor,
	rawHttpHonoActor,
	rawHttpNoHandlerActor,
	rawHttpVoidReturnActor,
} from "./raw-http";
import { rawHttpRequestPropertiesActor } from "./raw-http-request-properties";
import { rawWebSocketActor, rawWebSocketBinaryActor } from "./raw-websocket";
import { requestAccessActor } from "./request-access";
import { rejectConnectionActor } from "./reject-connection";
import {
	runWithError,
	runWithEarlyExit,
	runWithoutHandler,
	runWithQueueConsumer,
	runWithTicks,
} from "./run";
import { scheduled } from "./scheduled";
import {
	sleep,
	sleepWithLongRpc,
	sleepWithNoSleepOption,
	sleepWithRawHttp,
	sleepWithRawWebSocket,
} from "./sleep";
import { statelessActor } from "./stateless";
import {
	driverCtxActor,
	dynamicVarActor,
	nestedVarActor,
	staticVarActor,
	uniqueVarActor,
} from "./vars";
import {
	workflowAccessActor,
	workflowCounterActor,
	workflowQueueActor,
	workflowSleepActor,
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
		// From sleep.ts
		sleep,
		sleepWithLongRpc,
		sleepWithRawHttp,
		sleepWithRawWebSocket,
		sleepWithNoSleepOption,
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
		workflowSleepActor,
		workflowStopTeardownActor,
		// From actor-db-raw.ts
		dbActorRaw,
		// From actor-db-drizzle.ts
		dbActorDrizzle,
		// From stateless.ts
		statelessActor,
			// From access-control.ts
			accessControlActor,
			accessControlNoQueuesActor,
		},
	});

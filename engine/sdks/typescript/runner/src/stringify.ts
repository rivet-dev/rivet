import type * as protocol from "@rivetkit/engine-runner-protocol";
import { idToStr } from "./utils";

/**
 * Helper function to stringify ArrayBuffer for logging
 */
function stringifyArrayBuffer(buffer: ArrayBuffer): string {
	return `ArrayBuffer(${buffer.byteLength})`;
}

/**
 * Helper function to stringify bigint for logging
 */
function stringifyBigInt(value: bigint): string {
	return `${value}n`;
}

/**
 * Helper function to stringify Map for logging
 */
function stringifyMap(map: ReadonlyMap<string, string>): string {
	const entries = Array.from(map.entries())
		.map(([k, v]) => `"${k}": "${v}"`)
		.join(", ");
	return `Map(${map.size}){${entries}}`;
}

/**
 * Helper function to stringify MessageId for logging
 */
function stringifyMessageId(messageId: protocol.MessageId): string {
	return `MessageId{gatewayId: ${idToStr(messageId.gatewayId)}, requestId: ${idToStr(messageId.requestId)}, messageIndex: ${messageId.messageIndex}}`;
}

/**
 * Stringify ToServerTunnelMessageKind for logging
 * Handles ArrayBuffers, BigInts, and Maps that can't be JSON.stringified
 */
export function stringifyToServerTunnelMessageKind(
	kind: protocol.ToServerTunnelMessageKind,
): string {
	switch (kind.tag) {
		case "DeprecatedTunnelAck":
			return "DeprecatedTunnelAck";
		case "ToServerResponseStart": {
			const { status, headers, body, stream } = kind.val;
			const bodyStr = body === null ? "null" : stringifyArrayBuffer(body);
			return `ToServerResponseStart{status: ${status}, headers: ${stringifyMap(headers)}, body: ${bodyStr}, stream: ${stream}}`;
		}
		case "ToServerResponseChunk": {
			const { body, finish } = kind.val;
			return `ToServerResponseChunk{body: ${stringifyArrayBuffer(body)}, finish: ${finish}}`;
		}
		case "ToServerResponseAbort":
			return "ToServerResponseAbort";
		case "ToServerWebSocketOpen": {
			const { canHibernate } = kind.val;
			return `ToServerWebSocketOpen{canHibernate: ${canHibernate}}`;
		}
		case "ToServerWebSocketMessage": {
			const { data, binary } = kind.val;
			return `ToServerWebSocketMessage{data: ${stringifyArrayBuffer(data)}, binary: ${binary}}`;
		}
		case "ToServerWebSocketMessageAck": {
			const { index } = kind.val;
			return `ToServerWebSocketMessageAck{index: ${index}}`;
		}
		case "ToServerWebSocketClose": {
			const { code, reason, hibernate } = kind.val;
			const codeStr = code === null ? "null" : code.toString();
			const reasonStr = reason === null ? "null" : `"${reason}"`;
			return `ToServerWebSocketClose{code: ${codeStr}, reason: ${reasonStr}, hibernate: ${hibernate}}`;
		}
	}
}

/**
 * Stringify ToClientTunnelMessageKind for logging
 * Handles ArrayBuffers, BigInts, and Maps that can't be JSON.stringified
 */
export function stringifyToClientTunnelMessageKind(
	kind: protocol.ToClientTunnelMessageKind,
): string {
	switch (kind.tag) {
		case "DeprecatedTunnelAck":
			return "DeprecatedTunnelAck";
		case "ToClientRequestStart": {
			const { actorId, method, path, headers, body, stream } = kind.val;
			const bodyStr = body === null ? "null" : stringifyArrayBuffer(body);
			return `ToClientRequestStart{actorId: "${actorId}", method: "${method}", path: "${path}", headers: ${stringifyMap(headers)}, body: ${bodyStr}, stream: ${stream}}`;
		}
		case "ToClientRequestChunk": {
			const { body, finish } = kind.val;
			return `ToClientRequestChunk{body: ${stringifyArrayBuffer(body)}, finish: ${finish}}`;
		}
		case "ToClientRequestAbort":
			return "ToClientRequestAbort";
		case "ToClientWebSocketOpen": {
			const { actorId, path, headers } = kind.val;
			return `ToClientWebSocketOpen{actorId: "${actorId}", path: "${path}", headers: ${stringifyMap(headers)}}`;
		}
		case "ToClientWebSocketMessage": {
			const { data, binary } = kind.val;
			return `ToClientWebSocketMessage{data: ${stringifyArrayBuffer(data)}, binary: ${binary}}`;
		}
		case "ToClientWebSocketClose": {
			const { code, reason } = kind.val;
			const codeStr = code === null ? "null" : code.toString();
			const reasonStr = reason === null ? "null" : `"${reason}"`;
			return `ToClientWebSocketClose{code: ${codeStr}, reason: ${reasonStr}}`;
		}
	}
}

/**
 * Stringify Command for logging
 * Handles ArrayBuffers, BigInts, and Maps that can't be JSON.stringified
 */
export function stringifyCommand(command: protocol.Command): string {
	switch (command.tag) {
		case "CommandStartActor": {
			const { actorId, generation, config, hibernatingRequests } =
				command.val;
			const keyStr = config.key === null ? "null" : `"${config.key}"`;
			const inputStr =
				config.input === null
					? "null"
					: stringifyArrayBuffer(config.input);
			const hibernatingRequestsStr =
				hibernatingRequests.length > 0
					? `[${hibernatingRequests.map((hr) => `{gatewayId: ${idToStr(hr.gatewayId)}, requestId: ${idToStr(hr.requestId)}}`).join(", ")}]`
					: "[]";
			return `CommandStartActor{actorId: "${actorId}", generation: ${generation}, config: {name: "${config.name}", key: ${keyStr}, createTs: ${stringifyBigInt(config.createTs)}, input: ${inputStr}}, hibernatingRequests: ${hibernatingRequestsStr}}`;
		}
		case "CommandStopActor": {
			const { actorId, generation } = command.val;
			return `CommandStopActor{actorId: "${actorId}", generation: ${generation}}`;
		}
	}
}

/**
 * Stringify CommandWrapper for logging
 * Handles ArrayBuffers, BigInts, and Maps that can't be JSON.stringified
 */
export function stringifyCommandWrapper(
	wrapper: protocol.CommandWrapper,
): string {
	return `CommandWrapper{index: ${stringifyBigInt(wrapper.index)}, inner: ${stringifyCommand(wrapper.inner)}}`;
}

/**
 * Stringify Event for logging
 * Handles ArrayBuffers, BigInts, and Maps that can't be JSON.stringified
 */
export function stringifyEvent(event: protocol.Event): string {
	switch (event.tag) {
		case "EventActorIntent": {
			const { actorId, generation, intent } = event.val;
			const intentStr =
				intent.tag === "ActorIntentSleep"
					? "Sleep"
					: intent.tag === "ActorIntentStop"
						? "Stop"
						: "Unknown";
			return `EventActorIntent{actorId: "${actorId}", generation: ${generation}, intent: ${intentStr}}`;
		}
		case "EventActorStateUpdate": {
			const { actorId, generation, state } = event.val;
			let stateStr: string;
			if (state.tag === "ActorStateRunning") {
				stateStr = "Running";
			} else if (state.tag === "ActorStateStopped") {
				const { code, message } = state.val;
				const messageStr = message === null ? "null" : `"${message}"`;
				stateStr = `Stopped{code: ${code}, message: ${messageStr}}`;
			} else {
				stateStr = "Unknown";
			}
			return `EventActorStateUpdate{actorId: "${actorId}", generation: ${generation}, state: ${stateStr}}`;
		}
		case "EventActorSetAlarm": {
			const { actorId, generation, alarmTs } = event.val;
			const alarmTsStr =
				alarmTs === null ? "null" : stringifyBigInt(alarmTs);
			return `EventActorSetAlarm{actorId: "${actorId}", generation: ${generation}, alarmTs: ${alarmTsStr}}`;
		}
	}
}

/**
 * Stringify EventWrapper for logging
 * Handles ArrayBuffers, BigInts, and Maps that can't be JSON.stringified
 */
export function stringifyEventWrapper(wrapper: protocol.EventWrapper): string {
	return `EventWrapper{index: ${stringifyBigInt(wrapper.index)}, inner: ${stringifyEvent(wrapper.inner)}}`;
}

/**
 * Stringify ToServer for logging
 * Handles ArrayBuffers, BigInts, and Maps that can't be JSON.stringified
 */
export function stringifyToServer(message: protocol.ToServer): string {
	switch (message.tag) {
		case "ToServerInit": {
			const {
				name,
				version,
				totalSlots,
				lastCommandIdx,
				prepopulateActorNames,
				metadata,
			} = message.val;
			const lastCommandIdxStr =
				lastCommandIdx === null
					? "null"
					: stringifyBigInt(lastCommandIdx);
			const prepopulateActorNamesStr =
				prepopulateActorNames === null
					? "null"
					: `Map(${prepopulateActorNames.size})`;
			const metadataStr = metadata === null ? "null" : `"${metadata}"`;
			return `ToServerInit{name: "${name}", version: ${version}, totalSlots: ${totalSlots}, lastCommandIdx: ${lastCommandIdxStr}, prepopulateActorNames: ${prepopulateActorNamesStr}, metadata: ${metadataStr}}`;
		}
		case "ToServerEvents": {
			const events = message.val;
			return `ToServerEvents{count: ${events.length}, events: [${events.map((e) => stringifyEventWrapper(e)).join(", ")}]}`;
		}
		case "ToServerAckCommands": {
			const { lastCommandIdx } = message.val;
			return `ToServerAckCommands{lastCommandIdx: ${stringifyBigInt(lastCommandIdx)}}`;
		}
		case "ToServerStopping":
			return "ToServerStopping";
		case "ToServerPing": {
			const { ts } = message.val;
			return `ToServerPing{ts: ${stringifyBigInt(ts)}}`;
		}
		case "ToServerKvRequest": {
			const { actorId, requestId, data } = message.val;
			const dataStr = stringifyKvRequestData(data);
			return `ToServerKvRequest{actorId: "${actorId}", requestId: ${requestId}, data: ${dataStr}}`;
		}
		case "ToServerTunnelMessage": {
			const { messageId, messageKind } = message.val;
			return `ToServerTunnelMessage{messageId: ${stringifyMessageId(messageId)}, messageKind: ${stringifyToServerTunnelMessageKind(messageKind)}}`;
		}
	}
}

/**
 * Stringify ToClient for logging
 * Handles ArrayBuffers, BigInts, and Maps that can't be JSON.stringified
 */
export function stringifyToClient(message: protocol.ToClient): string {
	switch (message.tag) {
		case "ToClientInit": {
			const { runnerId, lastEventIdx, metadata } = message.val;
			const metadataStr = `{runnerLostThreshold: ${stringifyBigInt(metadata.runnerLostThreshold)}}`;
			return `ToClientInit{runnerId: "${runnerId}", lastEventIdx: ${stringifyBigInt(lastEventIdx)}, metadata: ${metadataStr}}`;
		}
		case "ToClientClose":
			return "ToClientClose";
		case "ToClientCommands": {
			const commands = message.val;
			return `ToClientCommands{count: ${commands.length}, commands: [${commands.map((c) => stringifyCommandWrapper(c)).join(", ")}]}`;
		}
		case "ToClientAckEvents": {
			const { lastEventIdx } = message.val;
			return `ToClientAckEvents{lastEventIdx: ${stringifyBigInt(lastEventIdx)}}`;
		}
		case "ToClientKvResponse": {
			const { requestId, data } = message.val;
			const dataStr = stringifyKvResponseData(data);
			return `ToClientKvResponse{requestId: ${requestId}, data: ${dataStr}}`;
		}
		case "ToClientTunnelMessage": {
			const { messageId, messageKind } = message.val;
			return `ToClientTunnelMessage{messageId: ${stringifyMessageId(messageId)}, messageKind: ${stringifyToClientTunnelMessageKind(messageKind)}}`;
		}
	}
}

/**
 * Stringify KvRequestData for logging
 */
function stringifyKvRequestData(data: protocol.KvRequestData): string {
	switch (data.tag) {
		case "KvGetRequest": {
			const { keys } = data.val;
			return `KvGetRequest{keys: ${keys.length}}`;
		}
		case "KvListRequest": {
			const { query, reverse, limit } = data.val;
			const reverseStr = reverse === null ? "null" : reverse.toString();
			const limitStr = limit === null ? "null" : stringifyBigInt(limit);
			return `KvListRequest{query: ${stringifyKvListQuery(query)}, reverse: ${reverseStr}, limit: ${limitStr}}`;
		}
		case "KvPutRequest": {
			const { keys, values } = data.val;
			return `KvPutRequest{keys: ${keys.length}, values: ${values.length}}`;
		}
		case "KvDeleteRequest": {
			const { keys } = data.val;
			return `KvDeleteRequest{keys: ${keys.length}}`;
		}
		case "KvDropRequest":
			return "KvDropRequest";
	}
}

/**
 * Stringify KvListQuery for logging
 */
function stringifyKvListQuery(query: protocol.KvListQuery): string {
	switch (query.tag) {
		case "KvListAllQuery":
			return "KvListAllQuery";
		case "KvListRangeQuery": {
			const { start, end, exclusive } = query.val;
			return `KvListRangeQuery{start: ${stringifyArrayBuffer(start)}, end: ${stringifyArrayBuffer(end)}, exclusive: ${exclusive}}`;
		}
		case "KvListPrefixQuery": {
			const { key } = query.val;
			return `KvListPrefixQuery{key: ${stringifyArrayBuffer(key)}}`;
		}
	}
}

/**
 * Stringify KvResponseData for logging
 */
function stringifyKvResponseData(data: protocol.KvResponseData): string {
	switch (data.tag) {
		case "KvErrorResponse": {
			const { message } = data.val;
			return `KvErrorResponse{message: "${message}"}`;
		}
		case "KvGetResponse": {
			const { keys, values, metadata } = data.val;
			return `KvGetResponse{keys: ${keys.length}, values: ${values.length}, metadata: ${metadata.length}}`;
		}
		case "KvListResponse": {
			const { keys, values, metadata } = data.val;
			return `KvListResponse{keys: ${keys.length}, values: ${values.length}, metadata: ${metadata.length}}`;
		}
		case "KvPutResponse":
			return "KvPutResponse";
		case "KvDeleteResponse":
			return "KvDeleteResponse";
		case "KvDropResponse":
			return "KvDropResponse";
	}
}

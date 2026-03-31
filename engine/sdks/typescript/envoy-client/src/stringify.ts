import type * as protocol from "@rivetkit/engine-envoy-protocol";
import { idToStr } from "./utils";

function stringifyArrayBuffer(buffer: ArrayBuffer): string {
	return `ArrayBuffer(${buffer.byteLength})`;
}

function stringifyBigInt(value: bigint): string {
	return `${value}n`;
}

function stringifyMap(map: ReadonlyMap<string, string>): string {
	const entries = Array.from(map.entries())
		.map(([k, v]) => `"${k}": "${v}"`)
		.join(", ");
	return `Map(${map.size}){${entries}}`;
}

function stringifyMessageId(messageId: protocol.MessageId): string {
	return `MessageId{gatewayId: ${idToStr(messageId.gatewayId)}, requestId: ${idToStr(messageId.requestId)}, messageIndex: ${messageId.messageIndex}}`;
}

export function stringifyToRivetTunnelMessageKind(
	kind: protocol.ToRivetTunnelMessageKind,
): string {
	switch (kind.tag) {
		case "ToRivetResponseStart": {
			const { status, headers, body, stream } = kind.val;
			const bodyStr = body === null ? "null" : stringifyArrayBuffer(body);
			return `ToRivetResponseStart{status: ${status}, headers: ${stringifyMap(headers)}, body: ${bodyStr}, stream: ${stream}}`;
		}
		case "ToRivetResponseChunk": {
			const { body, finish } = kind.val;
			return `ToRivetResponseChunk{body: ${stringifyArrayBuffer(body)}, finish: ${finish}}`;
		}
		case "ToRivetResponseAbort":
			return "ToRivetResponseAbort";
		case "ToRivetWebSocketOpen": {
			const { canHibernate } = kind.val;
			return `ToRivetWebSocketOpen{canHibernate: ${canHibernate}}`;
		}
		case "ToRivetWebSocketMessage": {
			const { data, binary } = kind.val;
			return `ToRivetWebSocketMessage{data: ${stringifyArrayBuffer(data)}, binary: ${binary}}`;
		}
		case "ToRivetWebSocketMessageAck": {
			const { index } = kind.val;
			return `ToRivetWebSocketMessageAck{index: ${index}}`;
		}
		case "ToRivetWebSocketClose": {
			const { code, reason, hibernate } = kind.val;
			const codeStr = code === null ? "null" : code.toString();
			const reasonStr = reason === null ? "null" : `"${reason}"`;
			return `ToRivetWebSocketClose{code: ${codeStr}, reason: ${reasonStr}, hibernate: ${hibernate}}`;
		}
	}
}

export function stringifyToEnvoyTunnelMessageKind(
	kind: protocol.ToEnvoyTunnelMessageKind,
): string {
	switch (kind.tag) {
		case "ToEnvoyRequestStart": {
			const { actorId, method, path, headers, body, stream } = kind.val;
			const bodyStr = body === null ? "null" : stringifyArrayBuffer(body);
			return `ToEnvoyRequestStart{actorId: "${actorId}", method: "${method}", path: "${path}", headers: ${stringifyMap(headers)}, body: ${bodyStr}, stream: ${stream}}`;
		}
		case "ToEnvoyRequestChunk": {
			const { body, finish } = kind.val;
			return `ToEnvoyRequestChunk{body: ${stringifyArrayBuffer(body)}, finish: ${finish}}`;
		}
		case "ToEnvoyRequestAbort":
			return "ToEnvoyRequestAbort";
		case "ToEnvoyWebSocketOpen": {
			const { actorId, path, headers } = kind.val;
			return `ToEnvoyWebSocketOpen{actorId: "${actorId}", path: "${path}", headers: ${stringifyMap(headers)}}`;
		}
		case "ToEnvoyWebSocketMessage": {
			const { data, binary } = kind.val;
			return `ToEnvoyWebSocketMessage{data: ${stringifyArrayBuffer(data)}, binary: ${binary}}`;
		}
		case "ToEnvoyWebSocketClose": {
			const { code, reason } = kind.val;
			const codeStr = code === null ? "null" : code.toString();
			const reasonStr = reason === null ? "null" : `"${reason}"`;
			return `ToEnvoyWebSocketClose{code: ${codeStr}, reason: ${reasonStr}}`;
		}
	}
}

export function stringifyCommand(command: protocol.Command): string {
	switch (command.tag) {
		case "CommandStartActor": {
			const { config, hibernatingRequests } = command.val;
			const keyStr = config.key === null ? "null" : `"${config.key}"`;
			const inputStr =
				config.input === null
					? "null"
					: stringifyArrayBuffer(config.input);
			const hibernatingRequestsStr =
				hibernatingRequests.length > 0
					? `[${hibernatingRequests.map((hr) => `{gatewayId: ${idToStr(hr.gatewayId)}, requestId: ${idToStr(hr.requestId)}}`).join(", ")}]`
					: "[]";
			return `CommandStartActor{config: {name: "${config.name}", key: ${keyStr}, createTs: ${stringifyBigInt(config.createTs)}, input: ${inputStr}}, hibernatingRequests: ${hibernatingRequestsStr}}`;
		}
		case "CommandStopActor": {
			const { reason } = command.val;
			return `CommandStopActor{reason: ${reason}}`;
		}
	}
}

export function stringifyCommandWrapper(
	wrapper: protocol.CommandWrapper,
): string {
	return `CommandWrapper{actorId: "${wrapper.checkpoint.actorId}", generation: "${wrapper.checkpoint.generation}", index: ${stringifyBigInt(wrapper.checkpoint.index)}, inner: ${stringifyCommand(wrapper.inner)}}`;
}

export function stringifyEvent(event: protocol.Event): string {
	switch (event.tag) {
		case "EventActorIntent": {
			const { intent } = event.val;
			const intentStr =
				intent.tag === "ActorIntentSleep"
					? "Sleep"
					: intent.tag === "ActorIntentStop"
						? "Stop"
						: "Unknown";
			return `EventActorIntent{intent: ${intentStr}}`;
		}
		case "EventActorStateUpdate": {
			const { state } = event.val;
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
			return `EventActorStateUpdate{state: ${stateStr}}`;
		}
		case "EventActorSetAlarm": {
			const { alarmTs } = event.val;
			const alarmTsStr =
				alarmTs === null ? "null" : stringifyBigInt(alarmTs);
			return `EventActorSetAlarm{alarmTs: ${alarmTsStr}}`;
		}
	}
}

export function stringifyEventWrapper(wrapper: protocol.EventWrapper): string {
	return `EventWrapper{actorId: ${wrapper.checkpoint.actorId}, generation: "${wrapper.checkpoint.generation}", index: ${stringifyBigInt(wrapper.checkpoint.index)}, inner: ${stringifyEvent(wrapper.inner)}}`;
}

export function stringifyToRivet(message: protocol.ToRivet): string {
	switch (message.tag) {
		case "ToRivetInit": {
			const { envoyKey, version, prepopulateActorNames, metadata } =
				message.val;
			const prepopulateActorNamesStr =
				prepopulateActorNames === null
					? "null"
					: `Map(${prepopulateActorNames.size})`;
			const metadataStr = metadata === null ? "null" : `"${metadata}"`;
			return `ToRivetInit{envoyKey: "${envoyKey}", version: ${version}, prepopulateActorNames: ${prepopulateActorNamesStr}, metadata: ${metadataStr}}`;
		}
		case "ToRivetEvents": {
			const events = message.val;
			return `ToRivetEvents{count: ${events.length}, events: [${events.map((e) => stringifyEventWrapper(e)).join(", ")}]}`;
		}
		case "ToRivetAckCommands": {
			const { lastCommandCheckpoints } = message.val;
			const checkpointsStr =
				lastCommandCheckpoints.length > 0
					? `[${lastCommandCheckpoints.map((cp) => `{actorId: "${cp.actorId}", index: ${stringifyBigInt(cp.index)}}`).join(", ")}]`
					: "[]";
			return `ToRivetAckCommands{lastCommandCheckpoints: ${checkpointsStr}}`;
		}
		case "ToRivetStopping":
			return "ToRivetStopping";
		case "ToRivetPong": {
			const { ts } = message.val;
			return `ToRivetPong{ts: ${stringifyBigInt(ts)}}`;
		}
		case "ToRivetKvRequest": {
			const { actorId, requestId, data } = message.val;
			const dataStr = stringifyKvRequestData(data);
			return `ToRivetKvRequest{actorId: "${actorId}", requestId: ${requestId}, data: ${dataStr}}`;
		}
		case "ToRivetTunnelMessage": {
			const { messageId, messageKind } = message.val;
			return `ToRivetTunnelMessage{messageId: ${stringifyMessageId(messageId)}, messageKind: ${stringifyToRivetTunnelMessageKind(messageKind)}}`;
		}
	}
}

export function stringifyToEnvoy(message: protocol.ToEnvoy): string {
	switch (message.tag) {
		case "ToEnvoyInit": {
			const { metadata } = message.val;
			const metadataStr = `{envoyLostThreshold: ${stringifyBigInt(metadata.envoyLostThreshold)}, actorStopThreshold: ${stringifyBigInt(metadata.actorStopThreshold)}, serverlessDrainGracePeriod: ${metadata.serverlessDrainGracePeriod === null ? "null" : stringifyBigInt(metadata.serverlessDrainGracePeriod)}, maxResponsePayloadSize: ${stringifyBigInt(metadata.maxResponsePayloadSize)}}`;
			return `ToEnvoyInit{metadata: ${metadataStr}}`;
		}
		case "ToEnvoyCommands": {
			const commands = message.val;
			return `ToEnvoyCommands{count: ${commands.length}, commands: [${commands.map((c) => stringifyCommandWrapper(c)).join(", ")}]}`;
		}
		case "ToEnvoyAckEvents": {
			const { lastEventCheckpoints } = message.val;
			const checkpointsStr =
				lastEventCheckpoints.length > 0
					? `[${lastEventCheckpoints.map((cp) => `{actorId: "${cp.actorId}", index: ${stringifyBigInt(cp.index)}}`).join(", ")}]`
					: "[]";
			return `ToEnvoyAckEvents{lastEventCheckpoints: ${checkpointsStr}}`;
		}
		case "ToEnvoyKvResponse": {
			const { requestId, data } = message.val;
			const dataStr = stringifyKvResponseData(data);
			return `ToEnvoyKvResponse{requestId: ${requestId}, data: ${dataStr}}`;
		}
		case "ToEnvoyTunnelMessage": {
			const { messageId, messageKind } = message.val;
			return `ToEnvoyTunnelMessage{messageId: ${stringifyMessageId(messageId)}, messageKind: ${stringifyToEnvoyTunnelMessageKind(messageKind)}}`;
		}
		case "ToEnvoyPing": {
			const { ts } = message.val;
			return `ToEnvoyPing{ts: ${stringifyBigInt(ts)}}`;
		}
	}
}

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
		case "KvDeleteRangeRequest": {
			const { start, end } = data.val;
			return `KvDeleteRangeRequest{start: ${stringifyArrayBuffer(start)}, end: ${stringifyArrayBuffer(end)}}`;
		}
		case "KvDropRequest":
			return "KvDropRequest";
	}
}

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

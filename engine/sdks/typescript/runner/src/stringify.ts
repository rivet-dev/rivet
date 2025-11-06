import type * as protocol from "@rivetkit/engine-runner-protocol";

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
 * Stringify ToServerTunnelMessageKind for logging
 * Handles ArrayBuffers, BigInts, and Maps that can't be JSON.stringified
 */
export function stringifyToServerTunnelMessageKind(
	kind: protocol.ToServerTunnelMessageKind,
): string {
	switch (kind.tag) {
		case "TunnelAck":
			return "TunnelAck";
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
			const { canHibernate, lastMsgIndex } = kind.val;
			return `ToServerWebSocketOpen{canHibernate: ${canHibernate}, lastMsgIndex: ${stringifyBigInt(lastMsgIndex)}}`;
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
			const { code, reason, retry } = kind.val;
			const codeStr = code === null ? "null" : code.toString();
			const reasonStr = reason === null ? "null" : `"${reason}"`;
			return `ToServerWebSocketClose{code: ${codeStr}, reason: ${reasonStr}, retry: ${retry}}`;
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
		case "TunnelAck":
			return "TunnelAck";
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
			const { index, data, binary } = kind.val;
			return `ToClientWebSocketMessage{index: ${index}, data: ${stringifyArrayBuffer(data)}, binary: ${binary}}`;
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
			const { actorId, generation, config } = command.val;
			const keyStr = config.key === null ? "null" : `"${config.key}"`;
			const inputStr =
				config.input === null
					? "null"
					: stringifyArrayBuffer(config.input);
			return `CommandStartActor{actorId: "${actorId}", generation: ${generation}, config: {name: "${config.name}", key: ${keyStr}, createTs: ${stringifyBigInt(config.createTs)}, input: ${inputStr}}}`;
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

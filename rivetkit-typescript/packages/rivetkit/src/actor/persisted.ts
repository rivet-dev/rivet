/** State object that gets automatically persisted to storage. */
export interface PersistedActor<S, CP, CS, I> {
	input?: I;
	hasInitiated: boolean;
	state: S;
	connections: PersistedConn<CP, CS>[];
	scheduledEvents: PersistedScheduleEvent[];
	hibernatableWebSocket: PersistedHibernatableWebSocket[];
}

/** Object representing connection that gets persisted to storage. */
export interface PersistedConn<CP, CS> {
	connId: string;
	token: string;
	params: CP;
	state: CS;
	subscriptions: PersistedSubscription[];

	/** Last time the socket was seen. This is set when disconencted so we can determine when we need to clean this up. */
	lastSeen: number;
}

export interface PersistedSubscription {
	eventName: string;
}

export interface GenericPersistedScheduleEvent {
	actionName: string;
	args: ArrayBuffer | null;
}

export type PersistedScheduleEventKind = {
	generic: GenericPersistedScheduleEvent;
};

export interface PersistedScheduleEvent {
	eventId: string;
	timestamp: number;
	kind: PersistedScheduleEventKind;
}

export interface PersistedHibernatableWebSocket {
	requestId: ArrayBuffer;
	lastSeenTimestamp: bigint;
	msgIndex: bigint;
}

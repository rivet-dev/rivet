import type { Rivet } from "@rivetkit/engine-api-full";
import type { Actor as InspectorActor } from "rivetkit/inspector";

export type { ActorLogEntry } from "rivetkit/inspector";
export { ActorFeature } from "rivetkit/inspector";

import type { ActorId } from "rivetkit/inspector";

export type { ActorId };

export type PortRouting = {
	guard?: {};
	host?: {};
};

export type Port = {
	protocol: "http" | "https" | "tcp" | "tcp_tls" | "udp";
	internalPort?: number;
	hostname?: string;
	port?: number;
	path?: string;
	/** Fully formed connection URL including protocol, hostname, port, and path, if applicable. */
	url?: string;
	routing: PortRouting;
};

export type Runtime = {
	build: string;
	arguments?: string[];
	environment?: Record<string, string>;
};

export type Lifecycle = {
	/** The duration to wait for in milliseconds before killing the actor. This should be set to a safe default, and can be overridden during a DELETE request if needed. */
	killTimeout?: number;
	/** If true, the actor will try to reschedule itself automatically in the event of a crash or a datacenter failover. The actor will not reschedule if it exits successfully. */
	durable?: boolean;
};

export type Resources = {
	/**
	 * The number of CPU cores in millicores, or 1/1000 of a core. For example,
	 * 1/8 of a core would be 125 millicores, and 1 core would be 1000
	 * millicores.
	 */
	cpu: number;
	/** The amount of memory in megabytes */
	memory: number;
};

export type Actor = Omit<InspectorActor, "id" | "key"> & {
	network?: {
		mode: "bridge" | "host";
		ports: Record<string, Port>;
	};
	runtime?: Runtime;
	lifecycle?: Lifecycle;
	key: string | undefined;

	// engine related
	runner?: string;
	crashPolicy?: CrashPolicy;
	sleepingAt?: string | null;
	connectableAt?: string | null;
	pendingAllocationAt?: string | null;
	datacenter?: string | null;
	rescheduleAt?: string | null;
	error?: Rivet.ActorError | null;
} & { id: ActorId };

export enum CrashPolicy {
	Restart = "restart",
	Sleep = "sleep",
	Destroy = "destroy",
}

export type ActorMetrics = {
	metrics: Record<string, number | null>;
	rawData: Record<string, number[]>;
	interval: number;
};

export type Build = {
	id: string;
	name: string;
};

export type Region = {
	id: string;
	name: string;
	url?: string;
};

export * from "./actor";

export type ActorStatus =
	| "starting"
	| "running"
	| "stopped"
	| "crashed"
	| "sleeping"
	| "pending"
	| "crash-loop"
	| "unknown";

export function getActorStatus(
	actor: Pick<
		Actor,
		| "createdAt"
		| "startedAt"
		| "connectableAt"
		| "destroyedAt"
		| "sleepingAt"
		| "pendingAllocationAt"
		| "rescheduleAt"
		| "error"
	>,
): ActorStatus {
	const {
		createdAt,
		connectableAt,
		destroyedAt,
		sleepingAt,
		pendingAllocationAt,
		rescheduleAt,
		error,
	} = actor;

	if (error) {
		return "crashed";
	}

	if (rescheduleAt) {
		return "crash-loop";
	}

	if (pendingAllocationAt && !connectableAt && !destroyedAt) {
		return "pending";
	}

	if (createdAt && sleepingAt && !destroyedAt) {
		return "sleeping";
	}

	if (createdAt && !connectableAt && !destroyedAt) {
		return "starting";
	}

	if (createdAt && connectableAt && !destroyedAt) {
		return "running";
	}

	if (createdAt && connectableAt && destroyedAt) {
		return "stopped";
	}

	if (createdAt && !connectableAt && destroyedAt) {
		return "crashed";
	}

	return "unknown";
}

import type { Actor as InspectorActor } from "@rivetkit/core/inspector";
import type { NamespaceNameId } from "@/queries/manager-engine";

export type { ActorLogEntry } from "@rivetkit/core/inspector";
export { ActorFeature } from "@rivetkit/core/inspector";

import type { ActorId } from "@rivetkit/core/inspector";

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
};

export * from "./actor";

export type ActorStatus =
	| "starting"
	| "running"
	| "stopped"
	| "crashed"
	| "sleeping"
	| "unknown";

export function getActorStatus(
	actor: Pick<
		Actor,
		"createdAt" | "startedAt" | "destroyedAt" | "sleepingAt"
	>,
): ActorStatus {
	const { createdAt, startedAt, destroyedAt, sleepingAt } = actor;

	if (createdAt && sleepingAt && !destroyedAt) {
		return "sleeping";
	}

	if (createdAt && !startedAt && !destroyedAt) {
		return "starting";
	}

	if (createdAt && startedAt && !destroyedAt) {
		return "running";
	}

	if (createdAt && startedAt && destroyedAt) {
		return "stopped";
	}

	if (createdAt && !startedAt && destroyedAt) {
		return "crashed";
	}

	return "unknown";
}

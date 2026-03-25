import type { Hono, Context as HonoContext } from "hono";
import type { ActorKey, Encoding, UniversalWebSocket } from "@/actor/mod";
import type { RegistryConfig } from "@/registry/config";
import type { GetUpgradeWebSocket } from "@/utils";
import type { ActorQuery, CrashPolicy } from "./protocol/query";

export type ManagerDriverBuilder = (config: RegistryConfig) => ManagerDriver;
export type GatewayTarget = { directId: string } | ActorQuery;

export interface ManagerDriver {
	getForId(input: GetForIdInput): Promise<ActorOutput | undefined>;
	getWithKey(input: GetWithKeyInput): Promise<ActorOutput | undefined>;
	getOrCreateWithKey(input: GetOrCreateWithKeyInput): Promise<ActorOutput>;
	createActor(input: CreateInput): Promise<ActorOutput>;
	listActors(input: ListActorsInput): Promise<ActorOutput[]>;

	sendRequest(
		target: GatewayTarget,
		actorRequest: Request,
	): Promise<Response>;
	openWebSocket(
		path: string,
		target: GatewayTarget,
		encoding: Encoding,
		params: unknown,
	): Promise<UniversalWebSocket>;
	proxyRequest(
		c: HonoContext,
		actorRequest: Request,
		actorId: string,
	): Promise<Response>;
	proxyWebSocket(
		c: HonoContext,
		path: string,
		actorId: string,
		encoding: Encoding,
		params: unknown,
	): Promise<Response>;

	/**
	 * Build a public gateway URL for a specific actor or query target.
	 *
	 * This lives on the driver because the base endpoint varies by runtime.
	 */
	buildGatewayUrl(target: GatewayTarget): Promise<string>;

	displayInformation(): ManagerDisplayInformation;

	extraStartupLog?: () => Record<string, unknown>;

	modifyManagerRouter?: (config: RegistryConfig, router: Hono) => void;
	/**
	 * Allows lazily setting getUpgradeWebSocket after the manager router has
	 * been initialized.
	 **/
	setGetUpgradeWebSocket(getUpgradeWebSocket: GetUpgradeWebSocket): void;

	/** Read a key. Returns null if the key doesn't exist. */
	kvGet(actorId: string, key: Uint8Array): Promise<string | null>;
}

export interface ManagerDisplayInformation {
	properties: Record<string, string>;
}

export interface GetForIdInput {
	c?: HonoContext | undefined;
	name: string;
	actorId: string;
}

export interface GetWithKeyInput {
	c?: HonoContext | undefined;
	name: string;
	key: ActorKey;
}

export interface GetOrCreateWithKeyInput {
	c?: HonoContext | undefined;
	name: string;
	key: ActorKey;
	input?: unknown;
	region?: string;
	crashPolicy?: CrashPolicy;
}

export interface CreateInput {
	c?: HonoContext | undefined;
	name: string;
	key: ActorKey;
	input?: unknown;
	region?: string;
	crashPolicy?: CrashPolicy;
}

export interface ListActorsInput {
	c?: HonoContext | undefined;
	name: string;
	key?: string;
	includeDestroyed?: boolean;
}

export interface ActorOutput {
	actorId: string;
	name: string;
	key: ActorKey;
	createTs?: number;
	startTs?: number | null;
	connectableTs?: number | null;
	sleepTs?: number | null;
	destroyTs?: number | null;
	error?: unknown;
}

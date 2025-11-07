import type { Env, Hono, Context as HonoContext } from "hono";
import type { ActorKey, Encoding, UniversalWebSocket } from "@/actor/mod";
import type { ManagerInspector } from "@/inspector/manager";
import type { RegistryConfig } from "@/registry/config";
import type { RunnerConfig } from "@/registry/run-config";

export type ManagerDriverBuilder = (
	registryConfig: RegistryConfig,
	runConfig: RunnerConfig,
) => ManagerDriver;

export interface ManagerDriver {
	getForId(input: GetForIdInput): Promise<ActorOutput | undefined>;
	getWithKey(input: GetWithKeyInput): Promise<ActorOutput | undefined>;
	getOrCreateWithKey(input: GetOrCreateWithKeyInput): Promise<ActorOutput>;
	createActor(input: CreateInput): Promise<ActorOutput>;

	sendRequest(actorId: string, actorRequest: Request): Promise<Response>;
	openWebSocket(
		path: string,
		actorId: string,
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

	displayInformation(): ManagerDisplayInformation;

	extraStartupLog?: () => Record<string, unknown>;

	modifyManagerRouter?: (
		registryConfig: RegistryConfig,
		router: Hono,
	) => void;

	// TODO(kacper): Remove this in favor of standard manager API
	/**
	 * @internal
	 */
	readonly inspector?: ManagerInspector;

	// TODO(kacper): Remove this in favor of ActorDriver.getinspectorToken
	/**
	 * Get or create the inspector access token.
	 * @experimental
	 * @returns creates or returns existing inspector access token
	 */
	getOrCreateInspectorAccessToken: () => string;
}

export interface ManagerDisplayInformation {
	name: string;
	properties: Record<string, string>;
}

export interface GetForIdInput<E extends Env = any> {
	c?: HonoContext | undefined;
	name: string;
	actorId: string;
}

export interface GetWithKeyInput<E extends Env = any> {
	c?: HonoContext | undefined;
	name: string;
	key: ActorKey;
}

export interface GetOrCreateWithKeyInput<E extends Env = any> {
	c?: HonoContext | undefined;
	name: string;
	key: ActorKey;
	input?: unknown;
	region?: string;
}

export interface CreateInput<E extends Env = any> {
	c?: HonoContext | undefined;
	name: string;
	key: ActorKey;
	input?: unknown;
	region?: string;
}

export interface ActorOutput {
	actorId: string;
	name: string;
	key: ActorKey;
}

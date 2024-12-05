export type ActorTags = Record<string, string>;

export interface RivetEnvironment {
	project?: string;
	environment?: string;
}

export function assertUnreachable(x: never): never {
    throw new Error(`Unreachable case: ${x}`);
}


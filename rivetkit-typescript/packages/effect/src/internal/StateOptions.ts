import type { Schema } from "effect";

export interface StateOptions<in out S extends Schema.Top> {
	readonly schema: S;
	readonly initialValue: () => S["Type"];
}

export interface Any {
	readonly schema: Schema.Top;
	readonly initialValue: () => unknown;
}

export type Encoded<State extends Any> =
	| State["schema"]["Encoded"]
	| ([State] extends [never] ? undefined : never);

export type Decoded<State extends Any> = State["schema"]["Type"];

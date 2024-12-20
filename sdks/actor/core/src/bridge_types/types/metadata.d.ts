// DO NOT MODIFY
//
// Generated from sdks/actor/bridge/

export interface Metadata {
	actor: {
		id: string;
		tags: Record<string, string>;
		createdAt: Date;
	};
	project: {
		id: string;
		slug: string;
	};
	environment: {
		id: string;
		slug: string;
	};
	cluster: {
		id: string;
	};
	region: {
		id: string;
		name: string;
	};
	build: {
		id: string;
	};
}

export type InKey = {
	jsInKey: Uint8Array[];
};
export type OutKey = {
	inKey: Uint8Array[];
	outKey: Uint8Array[];
};
export type OutEntry = {
	value: Uint8Array;
	metadata: KeyMetadata;
};
export type KeyMetadata = {
	kvVersion: Uint8Array;
	createTs: number;
};
export type ListQuery = {
	// Empty object
	all?: Record<string, null>;
	rangeInclusive?: [Uint8Array[], InKey];
	rangeExclusive?: [Uint8Array[], InKey];
	prefix?: Uint8Array[];
};

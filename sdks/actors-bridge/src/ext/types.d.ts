declare module "internal_types" {
	export type InKey = {
		jsInKey: Uint8Array[];
	};
	export type OutKey = {
		inKey: Uint8Array[];
		outKey: Uint8Array[];
	};
	export type OutEntry = {
		value: Uint8Array;
		metadata: Metadata;
	};
	export type Metadata = {
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
}

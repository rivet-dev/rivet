import { primordials } from "ext:core/mod.js";
import { KV_NAMESPACE } from "ext:rivet_kv/40_rivet_kv.js";
import { env } from "ext:runtime/30_os.js";
const { ObjectFreeze } = primordials;

export interface Metadata {
	actor: {
		id: string;
		tags: Record<string, string>;
		createdAt: Date;
	};
	env: {
		id: string;
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

function parseMetadata(): Metadata {
	let envMetadata = env.get("RIVET_METADATA");
	let metadata: Metadata | null = null;

	if (envMetadata) {
		try {
			let parsedMetadata = JSON.parse(envMetadata);

			metadata = {
				actor: {
					id: parsedMetadata.actor.id,
					tags: parsedMetadata.actor.tags,
					createdAt: new Date(parsedMetadata.actor.created_at),
				},
				env: {
					id: parsedMetadata.env.id,
				},
				cluster: {
					id: parsedMetadata.cluster.id,
				},
				region: {
					id: parsedMetadata.region.id,
					name: parsedMetadata.region.name,
				},
				build: {
					id: parsedMetadata.build.id,
				},
			};
		} catch (e) {
			console.warn("Rivet: failed to parse actor metadata:", e);
		}
	}

	// Fallback to defaults
	if (!metadata) {
		metadata = {
			actor: {
				id: null as any,
				tags: {},
				createdAt: new Date(0),
			},
			env: {
				id: null as any,
			},
			cluster: {
				id: null as any,
			},
			region: {
				id: null as any,
				name: null as any,
			},
			build: {
				id: null as any,
			},
		};
	}

	return metadata;
}

export const RIVET_NAMESPACE = {
	metadata: ObjectFreeze(parseMetadata()),
	kv: KV_NAMESPACE,
};

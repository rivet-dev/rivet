// DO NOT MODIFY
//
// Generated with scripts/pegboard/compile_bridge.ts

import { KV_NAMESPACE } from "ext:rivet_kv/40_rivet_kv.js";
import { env } from "ext:runtime/30_os.js";
function parseMetadata() {
    let envMetadata = env.get("RIVET_METADATA");
    let metadata = null;
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
                    name: parsedMetadata.region.name,
                },
                build: {
                    id: parsedMetadata.build.id,
                },
            };
        }
        catch (e) {
            console.warn("Rivet: failed to parse actor metadata:", e);
        }
    }
    // Fallback to defaults
    if (!metadata) {
        metadata = {
            actor: {
                id: null,
                tags: {},
                createdAt: new Date(0),
            },
            env: {
                id: null,
            },
            cluster: {
                id: null,
            },
            region: {
                name: null,
            },
            build: {
                id: null,
            },
        };
    }
    return metadata;
}
export const RIVET_NAMESPACE = {
    metadata: Object.freeze(parseMetadata()),
    kv: KV_NAMESPACE,
};

import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

import { NamespaceStatus } from "./NamespaceStatus";
import { NamespaceForkedFrom } from "./NamespaceForkedFrom";
import { NamespaceSkippedActor } from "./NamespaceSkippedActor";

export const Namespace: core.serialization.ObjectSchema<serializers.Namespace.Raw, Rivet.Namespace> = core.serialization.object({
    name: core.serialization.string(),
    createTs: core.serialization.property("create_ts", core.serialization.number()),
    status: NamespaceStatus,
    forkedFrom: core.serialization.property("forked_from", NamespaceForkedFrom.nullable()),
    archivedTs: core.serialization.property("archived_ts", core.serialization.number().nullable()),
    skipped: core.serialization.list(NamespaceSkippedActor).optional(),
    skippedCount: core.serialization.property("skipped_count", core.serialization.number().optional()),
});

export declare namespace Namespace {
    export interface Raw {
        name: string;
        create_ts: number;
        status: NamespaceStatus.Raw;
        forked_from: NamespaceForkedFrom.Raw | null;
        archived_ts: number | null;
        skipped?: NamespaceSkippedActor.Raw[] | null;
        skipped_count?: number | null;
    }
}

import * as Rivet from "../index";

export interface Namespace {
    /** Namespace name. */
    name: string;
    /** When the namespace was created, in Unix milliseconds. */
    createTs: number;
    /** `creating`, `forking`, `live`, `archiving`, `archived` or `failed`. */
    status: Rivet.NamespaceStatus;
    /** What the namespace was forked from; `null` for a namespace that is not a fork. */
    forkedFrom: Rivet.NamespaceForkedFrom | null;
    /** When the namespace was archived, in Unix milliseconds; `null` unless archived. */
    archivedTs: number | null;
    /** Legacy: the actors the move into the namespace's volume group could not move, at most 1,000; empty for every other namespace. Removed with the backfill in 3.1. */
    skipped?: Rivet.NamespaceSkippedActor[];
    /** Legacy: how many such actors are not destroyed yet, which can exceed the length of `skipped`. Removed with the backfill in 3.1. */
    skippedCount?: number;
}

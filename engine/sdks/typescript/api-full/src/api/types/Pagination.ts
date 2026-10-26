import * as Rivet from "../index";

export interface Pagination {
    /** Cursor for the next page, or `null` when there are no more results. */
    cursor: string | null;
}

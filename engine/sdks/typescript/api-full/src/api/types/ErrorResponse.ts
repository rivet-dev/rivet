import * as Rivet from "../index";

export interface ErrorResponse {
    /** Error group, for example `actor`. */
    group: string;
    /** Error code within the group, for example `not_found`. */
    code: string;
    /** Human-readable error message. */
    message: string;
    /** Extra details. Only present on some errors raised while authenticating or authorizing the request. */
    metadata?: unknown;
}

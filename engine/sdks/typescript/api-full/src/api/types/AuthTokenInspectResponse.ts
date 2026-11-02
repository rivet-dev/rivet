import * as Rivet from "../index";

export interface AuthTokenInspectResponse {
    /** Name of the namespace the token is scoped to. */
    namespace: string;
    /** Subject attached to the token, or `null`. */
    subject: string | null;
    /** Access the token carries. */
    grants: (Rivet.AuthTokenGrant)[];
    /** When the token was issued, in Unix milliseconds. */
    issuedTs: number;
    /** When the token expires, in Unix milliseconds. */
    expiresTs: number;
}

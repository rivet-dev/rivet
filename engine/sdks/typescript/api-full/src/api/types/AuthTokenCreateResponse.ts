import * as Rivet from "../index";

export interface AuthTokenCreateResponse {
    /** The signed JWT. */
    token: string;
    /** When the token was issued, in Unix milliseconds. */
    issuedTs: number;
    /** When the token expires, in Unix milliseconds. */
    expiresTs: number;
}

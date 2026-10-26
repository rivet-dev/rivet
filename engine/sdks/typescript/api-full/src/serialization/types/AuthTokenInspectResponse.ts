import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { AuthTokenGrant } from "./AuthTokenGrant";

export const AuthTokenInspectResponse: core.serialization.ObjectSchema<serializers.AuthTokenInspectResponse.Raw, Rivet.AuthTokenInspectResponse> = core.serialization.object({
    namespace: core.serialization.string(),
    subject: core.serialization.string() .nullable(),
    grants: core.serialization.list(AuthTokenGrant),
    issuedTs: core.serialization.property("issued_ts", core.serialization.number()),
    expiresTs: core.serialization.property("expires_ts", core.serialization.number()),
});

export declare namespace AuthTokenInspectResponse {
    export interface Raw {
        namespace: string;
        subject: string | null;
        grants: (AuthTokenGrant.Raw)[];
        issued_ts: number;
        expires_ts: number;
    }
}

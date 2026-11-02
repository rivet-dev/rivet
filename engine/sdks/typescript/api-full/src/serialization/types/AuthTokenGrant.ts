import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { AuthOperationKind } from "./AuthOperationKind";
import { AuthResourceKind } from "./AuthResourceKind";
import { AuthTargetScope } from "./AuthTargetScope";

export const AuthTokenGrant: core.serialization.ObjectSchema<serializers.AuthTokenGrant.Raw, Rivet.AuthTokenGrant> = core.serialization.object({
    resource: AuthResourceKind,
    target: AuthTargetScope,
    operations: core.serialization.list(AuthOperationKind),
});

export declare namespace AuthTokenGrant {
    export interface Raw {
        resource: AuthResourceKind.Raw;
        target: AuthTargetScope.Raw;
        operations: (AuthOperationKind.Raw)[];
    }
}

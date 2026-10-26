import * as Rivet from "../index";

export interface AuthTokenGrant {
    resource: Rivet.AuthResourceKind;
    target: Rivet.AuthTargetScope;
    /** Operations the grant allows on the target. */
    operations: (Rivet.AuthOperationKind)[];
}

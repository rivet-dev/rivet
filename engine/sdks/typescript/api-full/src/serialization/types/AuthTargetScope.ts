import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";
import { RivetId } from "./RivetId";

export const AuthTargetScope: core.serialization.Schema<serializers.AuthTargetScope.Raw, Rivet.AuthTargetScope> = core.serialization.undiscriminatedUnion([core.serialization.stringLiteral("any"), core.serialization.object({
    id: RivetId,
})]);

export declare namespace AuthTargetScope {
    export type Raw = "any" | { id: RivetId.Raw; };
}

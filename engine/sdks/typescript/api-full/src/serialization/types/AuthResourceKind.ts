import * as serializers from "../index";
import * as Rivet from "../../api/index";
import * as core from "../../core";

export const AuthResourceKind: core.serialization.Schema<serializers.AuthResourceKind.Raw, Rivet.AuthResourceKind> = core.serialization.enum_(["namespace", "actor", "node", "pool", "datacenter", "actor_gateway", "actor_kv"]);

export declare namespace AuthResourceKind {
    export type Raw = "namespace" | "actor" | "node" | "pool" | "datacenter" | "actor_gateway" | "actor_kv";
}

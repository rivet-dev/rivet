import * as Rivet from "../index";

export interface Namespace {
    /** Namespace name. */
    name: string;
    /** When the namespace was created, in Unix milliseconds. */
    createTs: number;
}

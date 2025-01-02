import { z } from "zod";

export const ProtocolFormatSchema = z.enum(["json", "cbor"]);

// export type ProtocolFormat = z.infer<typeof ProtocolFormatSchema>;  // Slow type

/**
 * Protocol format used to communicate between the client & actor.
 */
export type ProtocolFormat = "json" | "cbor";

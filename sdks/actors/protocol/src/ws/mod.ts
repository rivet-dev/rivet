import { z } from "zod";

export const ProtocolFormatSchema = z.enum(["json", "cbor"]);

export type ProtocolFormat = z.infer<typeof ProtocolFormatSchema>;

import { z } from "zod/v4";

export const RivetIdSchema = z.string();
export type RivetId = z.infer<typeof RivetIdSchema>;

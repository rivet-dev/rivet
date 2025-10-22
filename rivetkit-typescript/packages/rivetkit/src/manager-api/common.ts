import { z } from "zod";

export const RivetIdSchema = z.string();
export type RivetId = z.infer<typeof RivetIdSchema>;

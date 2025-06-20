import { z } from "zod";

export const StatusSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("starting"), data: z.object({}) }),
  z.object({ type: z.literal("running"), data: z.object({}) }),
  z.object({ type: z.literal("finishing"), data: z.object({}) }),
  z.object({ type: z.literal("converting"), data: z.object({}) }),
  z.object({ type: z.literal("uploading"), data: z.object({}) }),
  z.object({ type: z.literal("failure"), data: z.object({ reason: z.string() }) }),
  z.object({ type: z.literal("success"), data: z.object({ buildId: z.string() }) }),
]);

export type Status = z.infer<typeof StatusSchema>;

export const BuildRequestSchema = z.object({
  buildName: z.string(),
  dockerfilePath: z.string(),
  environmentId: z.string(),
});

export type BuildRequest = z.infer<typeof BuildRequestSchema>;

export const BuildEventSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("status"), data: StatusSchema }),
  z.object({ type: z.literal("log"), data: z.object({ line: z.string() }) }),
]);

export type BuildEvent = z.infer<typeof BuildEventSchema>;

export interface BuildInfo {
  id: string;
  status: Status;
  buildName?: string;
  dockerfilePath?: string;
  environmentId?: string;
  contextPath?: string;
  outputPath?: string;
  events: BuildEvent[];
  containerProcess?: any;
  createdAt: Date;
  downloadedAt?: Date;
  cleanupTimeout?: NodeJS.Timeout;
}

import { z } from "zod";
import { NO_SEP_CHAR_REGEX, UNIT_SEP_CHAR } from "./common";

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

const ILLEGAL_BUILD_ARG_KEY = /[\s'"\\]/g;
const BuildArgsSchema = z.string()
    .transform((str) => JSON.parse(str))
    .pipe(z.array(z.string()))
    .refine((arr) => {
        // Check each key=value pair to ensure keys have no spaces
        return arr.every(item => {
            const [key] = item.split('=');
            if (!key) return false;
            if (ILLEGAL_BUILD_ARG_KEY.test(key)) return false;
            if (item.includes(UNIT_SEP_CHAR)) return false;
            return true;
        });
    }, { message: "Argument key/value contains invalid character" })
    .transform((arr) => {
        const result: Record<string, string> = Object.create(null);
        // Convert array of strings to an object
        for (const item of arr) {
            const [key, ...valueParts] = item.split('=');
            const value = valueParts.join('=');
            
            if (key && value !== undefined) {
                result[key] = value;
            }
        }
        
        return result;
    });

export const BuildRequestSchema = z.object({
    buildName: z.string()
        .regex(NO_SEP_CHAR_REGEX, "buildName cannot contain special characters"),
    dockerfilePath: z.string()
        .regex(NO_SEP_CHAR_REGEX, "dockerfilePath cannot contain special characters"),
    environmentId: z.string()
        .regex(NO_SEP_CHAR_REGEX, "environmentId cannot contain special characters"),
    buildArgs: BuildArgsSchema,
    buildTarget: z.string()
        .regex(NO_SEP_CHAR_REGEX, "buildTarget cannot contain special characters")
        .optional(),
    context: z.instanceof(File)
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
    buildName: string;
    dockerfilePath: string;
    environmentId: string;
    contextPath: string;
    buildArgs: Record<string, string>;
    buildTarget?: string;
    outputPath: string;
    events: BuildEvent[];
    containerProcess?: any;
    createdAt: Date;
    downloadedAt?: Date;
    cleanupTimeout?: NodeJS.Timeout;
}

import { z } from "zod";

export type ReplErrorCode = "unsupported" | "runtime_error" | "syntax";

const CodeMessageSchema = z.object({
	type: z.literal("code"),
	data: z.string(),
	id: z.string(),
});
const InitMessageSchema = z.object({
	type: z.literal("init"),
	rpcs: z.array(z.string()).optional(),
	endpoint: z.string(),
	name: z.string(),
	id: z.string(),

	// engine related
	engineToken: z.string().optional(),
	namespace: z.string().optional(),
	runnerName: z.string().optional(),
	inspectorToken: z.string().nullish(),
});

const ActionResponseSchema = z.object({
	type: z.literal("actionResponse"),
	id: z.number(),
	data: z.object({
		result: z.any().optional(),
		error: z.string().optional(),
		success: z.boolean(),
	}),
});

export const MessageSchema = z.discriminatedUnion("type", [
	CodeMessageSchema,
	InitMessageSchema,
	ActionResponseSchema,
]);

export const FormattedCodeSchema = z
	.object({
		fg: z.string().optional(),
		tokens: z.array(
			z.array(
				z.object({
					content: z.string(),
					color: z.string().optional(),
				}),
			),
		),
	})
	.catch((ctx) => ctx.input);

export const LogSchema = z.object({
	method: z.union([z.literal("log"), z.literal("warn"), z.literal("error")]),
	data: z.array(z.any()).optional(),
	timestamp: z.string().optional(),
});

export const ResponseSchema = z.discriminatedUnion("type", [
	z.object({
		type: z.literal("error"),
		id: z.string().optional(),
		data: z.any(),
	}),
	z.object({
		type: z.literal("formatted"),
		id: z.string(),
		data: FormattedCodeSchema,
	}),
	z.object({
		type: z.literal("result"),
		id: z.string(),
		data: z.any().optional(),
	}),
	z.object({
		type: z.literal("log"),
		id: z.string(),
		data: LogSchema,
	}),
	z.object({
		type: z.literal("ready"),
	}),
	z.object({
		type: z.literal("invokeAction"),
		id: z.number(),
		data: z.object({
			name: z.string(),
			args: z.array(z.any()),
		}),
	}),
]);

export type Response = z.infer<typeof ResponseSchema>;
export type Message = z.infer<typeof MessageSchema>;
export type FormattedCode = z.infer<typeof FormattedCodeSchema>;
export type Log = z.infer<typeof LogSchema>;
export type InitMessage = z.infer<typeof InitMessageSchema>;
export type CodeMessage = z.infer<typeof CodeMessageSchema>;
export type ActionResponse = z.infer<typeof ActionResponseSchema>;

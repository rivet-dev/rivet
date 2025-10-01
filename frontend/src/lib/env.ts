import z from "zod";

export const cloudEnvSchema = z.object({
	VITE_APP_API_URL: z.string().url(),
	VITE_APP_CLOUD_API_URL: z.string().url(),
	VITE_CLERK_PUBLISHABLE_KEY: z.string(),
});

export const cloudEnv = () => cloudEnvSchema.parse(import.meta.env);

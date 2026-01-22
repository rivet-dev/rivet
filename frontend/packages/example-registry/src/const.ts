// NOTE: When modifying these options, make sure to update
// the documentation at website/src/content/docs/meta/submit-template.mdx

export const TECHNOLOGIES = [
	{ name: "rivet", displayName: "Rivet" },
	{ name: "react", displayName: "React" },
	{ name: "next-js", displayName: "Next.js" },
	{ name: "hono", displayName: "Hono" },
	{ name: "express", displayName: "Express" },
	{ name: "cloudflare-workers", displayName: "Cloudflare Workers" },
	{ name: "vercel", displayName: "Vercel" },
	{ name: "bun", displayName: "Bun" },
	{ name: "deno", displayName: "Deno" },
	{ name: "elysia", displayName: "Elysia" },
	{ name: "trpc", displayName: "tRPC" },
	{ name: "drizzle", displayName: "Drizzle" },
	{ name: "websocket", displayName: "WebSocket" },
	{ name: "typescript", displayName: "TypeScript" },
	{ name: "durable-streams", displayName: "Durable Streams" },
	{ name: "effect", displayName: "Effect" },
] as const;

export const TAGS = [
	{ name: "starter", displayName: "Starter" },
	{ name: "ai", displayName: "AI" },
	{ name: "real-time", displayName: "Real-time" },
	{ name: "database", displayName: "Database" },
	{ name: "gaming", displayName: "Gaming" },
	{ name: "experimental", displayName: "Experimental" },
	{ name: "functional", displayName: "Functional" },
] as const;

export type Technology = (typeof TECHNOLOGIES)[number]["name"];
export type Tag = (typeof TAGS)[number]["name"];
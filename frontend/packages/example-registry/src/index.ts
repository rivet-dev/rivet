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
] as const;

export const TAGS = [
	{ name: "quickstart", displayName: "Quickstart" },
	{ name: "real-time", displayName: "Real-time" },
	{ name: "database", displayName: "Database" },
	{ name: "ai", displayName: "AI" },
	{ name: "gaming", displayName: "Gaming" },
] as const;

export type Technology = (typeof TECHNOLOGIES)[number]["name"];
export type Tag = (typeof TAGS)[number]["name"];

// Re-export from generated file
export type { Template } from "./_gen";
export { templates } from "./_gen";

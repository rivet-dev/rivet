import Image from "next/image";
import Link from "next/link";
import bunLogo from "../images/platforms/bun.svg";
import cloudflareWorkersLogo from "../images/platforms/cloudflare-workers.svg";
import fileSystemLogo from "../images/platforms/file-system.svg";
import memoryLogo from "../images/platforms/memory.svg";
import nodejsLogo from "../images/platforms/nodejs.svg";
import redisLogo from "../images/platforms/redis.svg";

import reactLogo from "../images/clients/react.svg";
import rustLogo from "../images/clients/rust.svg";
// Client images
import typescriptLogo from "../images/clients/typescript.svg";

import betterAuthLogo from "../images/integrations/better-auth.svg";
import elysiaLogo from "../images/integrations/elysia.svg";
import expressLogo from "../images/integrations/express.svg";
// Integration images
import honoLogo from "../images/integrations/hono.svg";
import trpcLogo from "../images/integrations/trpc.svg";
import vitestLogo from "../images/integrations/vitest.svg";

export function PlatformIcons() {
	const platforms = [
		// {
		//   href: "/docs/cloud",
		//   src: rivetWhiteLogo,
		//   alt: "Rivet Platform",
		//   tooltip: "Rivet"
		// },
		{
			href: "/docs/actors/quickstart-backend",
			src: nodejsLogo,
			alt: "Node.js",
			tooltip: "Node.js",
		},
		{
			href: "/docs/actors/quickstart-backend",
			src: bunLogo,
			alt: "Bun",
			tooltip: "Bun",
		},
		{
			href: "/docs/cloud",
			src: redisLogo,
			alt: "Redis",
			tooltip: "Redis",
		},
		{
			href: "/docs/cloud",
			src: cloudflareWorkersLogo,
			alt: "Cloudflare Workers",
			tooltip: "Cloudflare Workers",
		},
		{
			href: "/docs/cloud",
			src: fileSystemLogo,
			alt: "File System",
			tooltip: "File System",
		},
		{
			href: "/docs/cloud",
			src: memoryLogo,
			alt: "Memory",
			tooltip: "Memory",
		},
		{
			href: "/docs/clients/javascript",
			src: typescriptLogo,
			alt: "TypeScript",
			tooltip: "TypeScript",
		},
		{
			href: "/docs/clients/rust",
			src: rustLogo,
			alt: "Rust",
			tooltip: "Rust (Client)",
		},
		{
			href: "/docs/clients/react",
			src: reactLogo,
			alt: "React",
			tooltip: "React",
		},
		{
			href: "/docs/integrations/hono",
			src: honoLogo,
			alt: "Hono",
			tooltip: "Hono",
		},
		{
			href: "/docs/integrations/express",
			src: expressLogo,
			alt: "Express",
			tooltip: "Express",
		},
		{
			href: "/docs/integrations/elysia",
			src: elysiaLogo,
			alt: "Elysia",
			tooltip: "Elysia",
		},
		{
			href: "/docs/integrations/trpc",
			src: trpcLogo,
			alt: "tRPC",
			tooltip: "tRPC",
		},
		{
			href: "/docs/integrations/better-auth",
			src: betterAuthLogo,
			alt: "Better Auth",
			tooltip: "Better Auth",
		},
		{
			href: "/docs/general/testing",
			src: vitestLogo,
			alt: "Vitest",
			tooltip: "Vitest",
		},
	];

	return (
		<div className="my-6 flex flex-col items-center max-w-2xl mx-auto">
			<div className="hero-bg-exclude text-white/60 text-sm font-medium mb-4">
				Supports
			</div>
			<div className="hero-bg-exclude flex flex-wrap justify-center max-w-[500px]">
				{platforms.map((platform, index) => (
					<Link
						key={index}
						href={platform.href}
						className="group relative flex items-center justify-center w-[60px] h-[60px] p-3 transition-all duration-200"
					>
						<Image
							src={platform.src}
							alt={platform.alt}
							width={32}
							height={32}
							className="object-contain grayscale opacity-50 group-hover:grayscale-0 group-hover:opacity-100 group-hover:scale-110 transition-all duration-200"
						/>
						<div className="absolute bottom-full left-1/2 transform -translate-x-1/2 mb-2 px-2 py-1 bg-gray-900 text-white text-xs rounded opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none whitespace-nowrap z-10">
							{platform.tooltip}
						</div>
					</Link>
				))}
			</div>
		</div>
	);
}

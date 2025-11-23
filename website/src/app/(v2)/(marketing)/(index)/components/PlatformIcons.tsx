import Link from "next/link";
import Image from "next/image";

// Platform images
import rivetWhiteLogo from "../images/platforms/rivet-white.svg";
import cloudflareWorkersLogo from "../images/platforms/cloudflare-workers.svg";
import vercelLogo from "../images/platforms/vercel.svg";
import nodejsLogo from "../images/platforms/nodejs.svg";
import bunLogo from "../images/platforms/bun.svg";
import denoLogo from "../images/platforms/deno.svg";
import redisLogo from "../images/platforms/redis.svg";
import postgresLogo from "../images/platforms/postgres.svg";
import awsLogo from "../images/platforms/aws-light.svg";
import railwayLogo from "../images/platforms/railway.svg";
import gcpLogo from "../images/platforms/gcp.svg";
import kubernetesLogo from "../images/platforms/kubernetes.svg";
import hetznerLogo from "../images/platforms/hetzner.svg";
import fileSystemLogo from "../images/platforms/file-system.svg";
import memoryLogo from "../images/platforms/memory.svg";

// Client images
import typescriptLogo from "../images/clients/typescript.svg";
import rustLogo from "../images/clients/rust.svg";
import reactLogo from "../images/clients/react.svg";
import nextjsLogo from "../images/clients/nextjs.svg";
import svelteLogo from "../images/clients/svelte.svg";

// Integration images
import honoLogo from "../images/integrations/hono.svg";
import expressLogo from "../images/integrations/express.svg";
import elysiaLogo from "../images/integrations/elysia.svg";
import trpcLogo from "../images/integrations/trpc.svg";
import betterAuthLogo from "../images/integrations/better-auth.svg";
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
			href: "/docs/actors/quickstart/backend",
			src: nodejsLogo,
			alt: "Node.js (Backend)",
			tooltip: "Node.js",
		},
		{
			href: "/docs/actors/quickstart/backend",
			src: bunLogo,
			alt: "Bun (Backend)",
			tooltip: "Bun",
		},
		{
			href: "/docs/actors/quickstart/backend",
			src: denoLogo,
			alt: "Deno (Backend)",
			tooltip: "Deno",
		},
		"SEPARATOR",
		//{
		//  href: "/docs/cloud",
		//  src: fileSystemLogo,
		//  alt: "File System",
		//  tooltip: "File System"
		//},
		//{
		//  href: "/docs/cloud",
		//  src: memoryLogo,
		//  alt: "Memory",
		//  tooltip: "Memory"
		//},
		//{
		//  href: "/docs/clients/javascript",
		//  src: typescriptLogo,
		//  alt: "TypeScript",
		//  tooltip: "TypeScript"
		//},
		{
			href: "/docs/actors/quickstart/react",
			src: reactLogo,
			alt: "React",
			tooltip: "React (Frontend)",
		},
		{
			href: "/docs/actors/quickstart/next-js",
			src: nextjsLogo,
			alt: "Next.js",
			tooltip: "Next.js (Frontend & Backend)",
		},
		{
			href: "https://github.com/rivet-dev/rivetkit/pull/1172",
			src: svelteLogo,
			alt: "Svelte",
			tooltip: "Svelte (Frontend)",
		},
		{
			href: "/docs/clients/rust",
			src: rustLogo,
			alt: "Rust",
			tooltip: "Rust (Client)",
		},
		// {
		// 	href: "/docs/integrations/hono",
		// 	src: honoLogo,
		// 	alt: "Hono",
		// 	tooltip: "Hono",
		// },
		// {
		// 	href: "/docs/integrations/express",
		// 	src: expressLogo,
		// 	alt: "Express",
		// 	tooltip: "Express",
		// },
		//{
		//  href: "/docs/integrations/elysia",
		//  src: elysiaLogo,
		//  alt: "Elysia",
		//  tooltip: "Elysia"
		//},
		// {
		// 	href: "/docs/integrations/trpc",
		// 	src: trpcLogo,
		// 	alt: "tRPC",
		// 	tooltip: "tRPC",
		// },
		// {
		// 	href: "/docs/integrations/better-auth",
		// 	src: betterAuthLogo,
		// 	alt: "Better Auth",
		// 	tooltip: "Better Auth",
		// },
		//{
		//  href: "/docs/general/testing",
		//  src: vitestLogo,
		//  alt: "Vitest",
		//  tooltip: "Vitest"
		//}
		"SEPARATOR",
		{
			href: "https://github.com/rivet-dev/rivetkit/tree/67e8e26b1fdb22dcb4997a7f0a1dfb1461d7b3e7/examples/next-js",
			src: vercelLogo,
			alt: "Vercel Functions",
			tooltip: "Vercel Functions",
		},
		{
			href: "https://railway.com/deploy/rivet",
			src: railwayLogo,
			alt: "Railway",
			tooltip: "Railway",
		},
		{
			href: "/docs/actors/quickstart/cloudflare-workers",
			src: cloudflareWorkersLogo,
			alt: "Cloudflare Durable Objects",
			tooltip: "Cloudflare Durable Objects",
		},
		{
			href: "/docs/connect/kubernetes",
			src: kubernetesLogo,
			alt: "Kubernetes",
			tooltip: "Kubernetes",
		},
		{
			href: "/docs/connect/aws-ecs",
			src: awsLogo,
			alt: "AWS ECS",
			tooltip: "AWS ECS",
		},
		{
			href: "/docs/connect/gcp-cloud-run",
			src: gcpLogo,
			alt: "GCP Cloud Run",
			tooltip: "GCP Cloud Run",
		},
		{
			href: "/docs/connect/hetzner",
			src: hetznerLogo,
			alt: "Hetzner",
			tooltip: "Hetzner",
		},
	];

	return (
		<div className="my-6 flex flex-col items-center w-full">
			<div className="hero-bg-exclude text-white/30 text-xs font-medium mb-3">
				Supports
			</div>
			<div className="hero-bg-exclude flex flex-wrap justify-center">
				{platforms.map((platform, index) => {
					if (platform === "SEPARATOR") {
						return (
							<div
								key={index}
								className="flex items-center justify-center w-[50px] h-[50px]"
							>
								<div className="w-px h-[40px] bg-white/10" />
							</div>
						);
					}
					return (
						<Link
							key={index}
							href={platform.href}
							className="group relative flex items-center justify-center w-[50px] h-[50px] p-3 transition-all duration-200"
						>
							<Image
								src={platform.src}
								alt={platform.alt}
								width={32}
								height={32}
								className="object-contain grayscale opacity-30 group-hover:grayscale-0 group-hover:opacity-100 group-hover:scale-110 transition-all duration-200"
							/>
							<div className="absolute bottom-full left-1/2 transform -translate-x-1/2 mb-2 px-2 py-1 bg-background border border-white/10 text-white text-xs rounded opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none whitespace-nowrap z-10">
								{platform.tooltip}
							</div>
						</Link>
					);
				})}
			</div>
		</div>
	);
}

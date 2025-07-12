import Image from "next/image";
import Link from "next/link";
import awsLambdaLogo from "../images/platforms/aws-lambda.svg";
import bunLogo from "../images/platforms/bun.svg";
import cloudflareWorkersLogo from "../images/platforms/cloudflare-workers.svg";
import fileSystemLogo from "../images/platforms/file-system.svg";
import memoryLogo from "../images/platforms/memory.svg";
import nodejsLogo from "../images/platforms/nodejs.svg";
import postgresLogo from "../images/platforms/postgres.svg";
import redisLogo from "../images/platforms/redis.svg";
import supabaseLogo from "../images/platforms/supabase.svg";
import vercelLogo from "../images/platforms/vercel.svg";

import javascriptLogo from "../images/clients/javascript.svg";
import nextjsLogo from "../images/clients/nextjs.svg";
// Client images
import reactLogo from "../images/clients/react.svg";
import rustLogo from "../images/clients/rust.svg";
import typescriptLogo from "../images/clients/typescript.svg";
import vueLogo from "../images/clients/vue.svg";

import betterAuthLogo from "../images/integrations/better-auth.svg";
import elysiaLogo from "../images/integrations/elysia.svg";
import expressLogo from "../images/integrations/express.svg";
// Integration images
import honoLogo from "../images/integrations/hono.svg";
import livestoreLogo from "../images/integrations/livestore.svg";
import tinybaseLogo from "../images/integrations/tinybase.svg";
import vitestLogo from "../images/integrations/vitest.svg";
import yjsLogo from "../images/integrations/yjs.svg";
import zerosyncLogo from "../images/integrations/zerosync.svg";

interface TechLinkProps {
	href: string;
	name: string;
	icon: string;
	alt: string;
	external?: boolean;
	status?: "coming-soon" | "help-wanted" | "available-in-july";
}

function TechLink({ href, name, icon, alt, external, status }: TechLinkProps) {
	const baseClasses =
		"relative flex items-center gap-2.5 px-3 py-2.5 bg-white/2 border border-white/20 rounded-lg hover:bg-white/10 hover:border-white/40 transition-all duration-200 group";

	const linkProps = external
		? {
				target: "_blank",
				rel: "noopener noreferrer",
			}
		: {};

	const statusText =
		status === "coming-soon"
			? "On The Roadmap"
			: status === "help-wanted"
				? "Help Wanted"
				: status === "available-in-july"
					? "Available In July"
					: "";
	const statusClass =
		status === "coming-soon" || status === "available-in-july"
			? "bg-[#ff4f00] text-white"
			: status === "help-wanted"
				? "bg-[#0059ff] text-white"
				: "";

	return (
		<Link href={href} className={baseClasses} {...linkProps}>
			{status && (
				<span
					className={`absolute -top-1.5 -right-1.5 text-[10px] px-1.5 py-0.5 rounded ${statusClass} font-medium`}
				>
					{statusText}
				</span>
			)}
			<Image
				src={icon}
				alt={alt}
				width={22}
				height={22}
				className="object-contain"
			/>
			<span className="text-white text-sm font-medium">{name}</span>
		</Link>
	);
}

interface TechSubSectionProps {
	title: string;
	children: React.ReactNode;
}

function TechSubSection({ title, children }: TechSubSectionProps) {
	return (
		<div className="ml-auto max-w-md">
			<h3 className="text-lg font-600 text-white/80 mb-3">{title}</h3>
			<div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
				{children}
			</div>
		</div>
	);
}

export function TechSection() {
	return (
		<div className="mx-auto max-w-7xl">
			<div className="space-y-28">
				{/* Runs Anywhere Section */}
				<div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-start">
					<div className="space-y-6">
						<h2 className="text-4xl sm:text-5xl font-700 text-white">
							Runs Anywhere
						</h2>
						<div className="space-y-4">
							<p className="text-lg font-500 text-white/40 leading-relaxed">
								Deploy Rivet Actors anywhere - from serverless
								platforms to your own infrastructure with
								Rivet's flexible runtime options.
							</p>
							<p className="text-lg font-500 text-white/40 leading-relaxed">
								Don't see the runtime you want?{" "}
								<Link
									href="/docs/cloud"
									className="text-white/80 hover:text-white transition-colors underline"
								>
									Add your own
								</Link>
								.
							</p>
						</div>
					</div>

					<div className="space-y-8">
						<TechSubSection title="Compute">
							<TechLink
								href="/docs/actors/quickstart-backend"
								name="Node.js"
								icon={nodejsLogo}
								alt="Node.js"
							/>
							<TechLink
								href="/docs/actors/quickstart-backend"
								name="Bun"
								icon={bunLogo}
								alt="Bun"
							/>
							{/*<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/905"
								name="Deno"
								icon={denoLogo}
								alt="Deno"
								external
								status="help-wanted"
							/>*/}
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/897"
								name="Vercel"
								icon={vercelLogo}
								alt="Vercel"
								external
								status="coming-soon"
							/>
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/898"
								name="AWS Lambda"
								icon={awsLambdaLogo}
								alt="AWS Lambda"
								external
								status="coming-soon"
							/>
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/905"
								name="Supabase"
								icon={supabaseLogo}
								alt="Supabase"
								external
								status="help-wanted"
							/>
							<TechLink
								href="/docs/actors/quickstart-backend"
								name="Cloudflare"
								icon={cloudflareWorkersLogo}
								alt="Cloudflare"
							/>
						</TechSubSection>

						<TechSubSection title="Storage">
							<TechLink
								href="/docs/actors/quickstart-backend"
								name="Redis"
								icon={redisLogo}
								alt="Redis"
							/>
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/899"
								name="Postgres"
								icon={postgresLogo}
								alt="Postgres"
								external
								status="coming-soon"
							/>
							<TechLink
								href="/docs/actors/quickstart-backend"
								name="File System"
								icon={fileSystemLogo}
								alt="File System"
							/>
							<TechLink
								href="/docs/actors/quickstart-backend"
								name="Memory"
								icon={memoryLogo}
								alt="Memory"
							/>
						</TechSubSection>
					</div>
				</div>

				{/* Works With Your Tools Section */}
				<div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-start">
					<div className="space-y-6">
						<h2 className="text-4xl sm:text-5xl font-700 text-white">
							Works With Your Tools
						</h2>
						<div className="space-y-4">
							<p className="text-lg font-500 text-white/40 leading-relaxed">
								Seamlessly integrate Rivet with your favorite
								frameworks, languages, and tools.
							</p>
							<p className="text-lg font-500 text-white/40 leading-relaxed">
								Don't see what you need?{" "}
								<Link
									href="https://github.com/rivet-gg/rivetkit/issues/new"
									target="_blank"
									rel="noopener noreferrer"
									className="text-white/80 hover:text-white transition-colors underline"
								>
									Request an integration
								</Link>
								.
							</p>
						</div>
					</div>

					<div className="space-y-8">
						<TechSubSection title="Frameworks">
							<TechLink
								href="/docs/clients/react"
								name="React"
								icon={reactLogo}
								alt="React"
							/>
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/904"
								name="Next.js"
								icon={nextjsLogo}
								alt="Next.js"
								external
								status="help-wanted"
							/>
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/903"
								name="Vue"
								icon={vueLogo}
								alt="Vue"
								external
								status="help-wanted"
							/>
						</TechSubSection>

						<TechSubSection title="Clients">
							<TechLink
								href="/docs/clients/javascript"
								name="JavaScript"
								icon={javascriptLogo}
								alt="JavaScript"
							/>
							<TechLink
								href="/docs/clients/javascript"
								name="TypeScript"
								icon={typescriptLogo}
								alt="TypeScript"
							/>
							<TechLink
								href="/docs/clients/rust"
								name="Rust"
								icon={rustLogo}
								alt="Rust"
							/>
						</TechSubSection>

						<TechSubSection title="Integrations">
							<TechLink
								href="/docs/integrations/hono"
								name="Hono"
								icon={honoLogo}
								alt="Hono"
							/>
							<TechLink
								href="/docs/integrations/express"
								name="Express"
								icon={expressLogo}
								alt="Express"
							/>
							<TechLink
								href="/docs/integrations/elysia"
								name="Elysia"
								icon={elysiaLogo}
								alt="Elysia"
							/>
							<TechLink
								href="/docs/general/testing"
								name="Vitest"
								icon={vitestLogo}
								alt="Vitest"
							/>
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/906"
								name="Better Auth"
								icon={betterAuthLogo}
								alt="Better Auth"
								external
							/>
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/907"
								name="AI SDK"
								icon={vercelLogo}
								alt="AI SDK"
								external
								status="coming-soon"
							/>
						</TechSubSection>

						<TechSubSection title="Local-First Sync">
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/908"
								name="LiveStore"
								icon={livestoreLogo}
								alt="LiveStore"
								external
								status="coming-soon"
							/>
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/909"
								name="ZeroSync"
								icon={zerosyncLogo}
								alt="ZeroSync"
								external
								status="help-wanted"
							/>
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/910"
								name="TinyBase"
								icon={tinybaseLogo}
								alt="TinyBase"
								external
								status="help-wanted"
							/>
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/911"
								name="Yjs"
								icon={yjsLogo}
								alt="Yjs"
								external
								status="help-wanted"
							/>
						</TechSubSection>
					</div>
				</div>
			</div>
		</div>
	);
}

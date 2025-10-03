import Link from "next/link";
import Image from "next/image";

// Platform images
import rivetWhiteLogo from "../images/platforms/rivet-white.svg";
import cloudflareWorkersLogo from "../images/platforms/cloudflare-workers.svg";
import bunLogo from "../images/platforms/bun.svg";
import nodejsLogo from "../images/platforms/nodejs.svg";
import fileSystemLogo from "../images/platforms/file-system.svg";
import memoryLogo from "../images/platforms/memory.svg";
import vercelLogo from "../images/platforms/vercel.svg";
import awsLambdaLogo from "../images/platforms/aws-lambda.svg";
import supabaseLogo from "../images/platforms/supabase.svg";
import postgresLogo from "../images/platforms/postgres.svg";
import railwayLogo from "../images/platforms/railway.svg";

// Client images
import reactLogo from "../images/clients/react.svg";
import javascriptLogo from "../images/clients/javascript.svg";
import typescriptLogo from "../images/clients/typescript.svg";
import rustLogo from "../images/clients/rust.svg";
import nextjsLogo from "../images/clients/nextjs.svg";
import vueLogo from "../images/clients/vue.svg";

// Integration images
import honoLogo from "../images/integrations/hono.svg";
import expressLogo from "../images/integrations/express.svg";
import elysiaLogo from "../images/integrations/elysia.svg";
import trpcLogo from "../images/integrations/trpc.svg";
import vitestLogo from "../images/integrations/vitest.svg";
import betterAuthLogo from "../images/integrations/better-auth.svg";
import livestoreLogo from "../images/integrations/livestore.svg";
import zerosyncLogo from "../images/integrations/zerosync.svg";
import tinybaseLogo from "../images/integrations/tinybase.svg";
import yjsLogo from "../images/integrations/yjs.svg";

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
		<div className="mx-auto lg:ml-auto max-w-full lg:max-w-md">
			<h3 className="text-lg font-600 text-white/80 mb-3">{title}</h3>
			<div className="grid grid-cols-1 grid-cols-2 md:grid-cols-4 lg:grid-cols-2 gap-2.5">
				{children}
			</div>
		</div>
	);
}

interface TechSectionGroupProps {
	children: React.ReactNode;
}

function TechSectionGroup({ children }: TechSectionGroupProps) {
	return (
		<div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-start">
			{children}
		</div>
	);
}

interface TechSectionTextProps {
	heading: string;
	description: string;
	linkText: string;
	linkHref: string;
	linkExternal?: boolean;
}

function TechSectionText({
	heading,
	description,
	linkText,
	linkHref,
	linkExternal,
}: TechSectionTextProps) {
	const linkProps = linkExternal
		? {
				target: "_blank",
				rel: "noopener noreferrer",
			}
		: {};

	return (
		<div className="space-y-6">
			<h2 className="text-4xl sm:text-5xl font-700 text-white">
				{heading}
			</h2>
			<div className="space-y-4">
				<p className="text-lg font-500 text-white/40 leading-relaxed">
					{description}
				</p>
				<p className="text-lg font-500 text-white/40 leading-relaxed">
					Don't see what you need?{" "}
					<Link
						href={linkHref}
						className="text-white/80 hover:text-white transition-colors underline"
						{...linkProps}
					>
						{linkText}
					</Link>
					.
				</p>
			</div>
		</div>
	);
}

interface TechSectionSubsectionsProps {
	children: React.ReactNode;
}

function TechSectionSubsections({ children }: TechSectionSubsectionsProps) {
	return <div className="space-y-8">{children}</div>;
}

export function TechSection() {
	return (
		<div className="mx-auto max-w-7xl">
			<div className="space-y-28">
				<TechSectionGroup>
					<TechSectionText
						heading="Runs Anywhere"
						description="Deploy Rivet Actors anywhere - from serverless platforms to your own infrastructure with Rivet's flexible runtime options."
						linkText="Add your own"
						linkHref="/docs/cloud"
					/>

					<TechSectionSubsections>
						<TechSubSection title="Compute">
							<TechLink
								href="/docs/actors/quickstart/backend"
								name="Node.js"
								icon={nodejsLogo}
								alt="Node.js"
							/>
							<TechLink
								href="/docs/actors/quickstart/backend"
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
								href="https://railway.com/deploy/rivet"
								name="Railway"
								icon={railwayLogo}
								alt="Railway"
								external
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
								href="/docs/actors/quickstart/backend"
								name="Durable Objects"
								icon={cloudflareWorkersLogo}
								alt="Cloudflare Durable Objects"
							/>
						</TechSubSection>

						<TechSubSection title="Storage">
							<TechLink
								href="/docs/actors/"
								name="Postgres"
								icon={postgresLogo}
								alt="Postgres"
							/>
							<TechLink
								href="/docs/actors/quickstart/backend"
								name="File System"
								icon={fileSystemLogo}
								alt="File System"
							/>
							<TechLink
								href="/docs/actors/quickstart/backend"
								name="Memory"
								icon={memoryLogo}
								alt="Memory"
							/>
						</TechSubSection>
					</TechSectionSubsections>
				</TechSectionGroup>

				<TechSectionGroup>
					<TechSectionText
						heading="Works With Your Tools"
						description="Seamlessly integrate Rivet with your favorite frameworks, languages, and tools."
						linkText="Request an integration"
						linkHref="https://github.com/rivet-gg/rivetkit/issues/new"
						linkExternal
					/>

					<TechSectionSubsections>
						<TechSubSection title="Frontend & Clients">
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
								href="/docs/clients/react"
								name="React"
								icon={reactLogo}
								alt="React"
							/>
							<TechLink
								href="/docs/clients/rust"
								name="Rust"
								icon={rustLogo}
								alt="Rust"
							/>
							<TechLink
								href="/docs/clients/next-js"
								name="Next.js"
								icon={nextjsLogo}
								alt="Next.js"
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

						<TechSubSection title="Backend">
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
								href="/docs/integrations/trpc"
								name="tRPC"
								icon={trpcLogo}
								alt="tRPC"
							/>
						</TechSubSection>

						<TechSubSection title="Auth">
							<TechLink
								href="/docs/integrations/better-auth"
								name="Better Auth"
								icon={betterAuthLogo}
								alt="Better Auth"
							/>
						</TechSubSection>

						<TechSubSection title="Testing">
							<TechLink
								href="/docs/integrations/vitest"
								name="Vitest"
								icon={vitestLogo}
								alt="Vitest"
							/>
						</TechSubSection>

						<TechSubSection title="AI">
							<TechLink
								href="https://github.com/rivet-gg/rivetkit/issues/907"
								name="AI SDK"
								icon={vercelLogo}
								alt="AI SDK"
								external
								status="coming-soon"
							/>
						</TechSubSection>

						<TechSubSection title="Sync">
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
					</TechSectionSubsections>
				</TechSectionGroup>
			</div>
		</div>
	);
}

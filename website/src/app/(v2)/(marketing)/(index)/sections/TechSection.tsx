import Link from "next/link";
import Image from "next/image";

// Platform images
import rivetWhiteLogo from "../images/platforms/rivet-white.svg";
import cloudflareWorkersLogo from "../images/platforms/cloudflare-workers.svg";
import bunLogo from "../images/platforms/bun.svg";
import denoLogo from "../images/platforms/deno.svg";
import nodejsLogo from "../images/platforms/nodejs.svg";
import fileSystemLogo from "../images/platforms/file-system.svg";
import memoryLogo from "../images/platforms/memory.svg";
import vercelLogo from "../images/platforms/vercel.svg";
import awsLambdaLogo from "../images/platforms/aws-lambda.svg";
import awsLogo from "../images/platforms/aws-light.svg";
import supabaseLogo from "../images/platforms/supabase.svg";
import postgresLogo from "../images/platforms/postgres.svg";
import railwayLogo from "../images/platforms/railway.svg";
import freestyleLogo from "../images/platforms/freestyle.svg";
import gcpLogo from "../images/platforms/gcp.svg";
import kubernetesLogo from "../images/platforms/kubernetes.svg";
import hetznerLogo from "../images/platforms/hetzner.svg";
import vmBareMetalLogo from "../images/platforms/vm-bare-metal.svg";

// Client images
import reactLogo from "../images/clients/react.svg";
import javascriptLogo from "../images/clients/javascript.svg";
import typescriptLogo from "../images/clients/typescript.svg";
import rustLogo from "../images/clients/rust.svg";
import nextjsLogo from "../images/clients/nextjs.svg";
import vueLogo from "../images/clients/vue.svg";
import svelteLogo from "../images/clients/svelte.svg";

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
	status?: "coming-soon" | "help-wanted" | "1-click-deploy";
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
				: status === "1-click-deploy"
					? "1-Click Deploy"
					: "";
	const statusClass =
		status === "coming-soon"
			? "bg-[#ff4f00] text-white"
			: status === "help-wanted"
				? "bg-[#0059ff] text-white"
				: status === "1-click-deploy"
					? "bg-[#007aff] text-white"
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
						<TechSubSection title="Storage">
							<TechLink
								href="/docs/cloud"
								name="Rivet Cloud"
								icon={rivetWhiteLogo}
								alt="Rivet Cloud"
								status="1-click-deploy"
							/>
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
							<TechLink
								href="https://github.com/rivet-dev/rivetkit/tree/9a3d850aee45167eadf249fdbae60129bf37e818/examples/deno"
								name="Deno"
								icon={denoLogo}
								alt="Deno"
							/>
							<TechLink
								href="/docs/connect/vercel"
								name="Vercel"
								icon={vercelLogo}
								alt="Vercel"
								status="1-click-deploy"
							/>
							<TechLink
								href="https://railway.com/deploy/rivet"
								name="Railway"
								icon={railwayLogo}
								alt="Railway"
								external
								status="1-click-deploy"
							/>
							<TechLink
								href="/docs/actors/quickstart/backend"
								name="Durable Objects"
								icon={cloudflareWorkersLogo}
								alt="Cloudflare Durable Objects"
							/>
							<TechLink
								href="/docs/connect/kubernetes"
								name="Kubernetes"
								icon={kubernetesLogo}
								alt="Kubernetes"
							/>
							<TechLink
								href="/docs/connect/aws-ecs"
								name="AWS ECS"
								icon={awsLogo}
								alt="AWS ECS"
							/>
							<TechLink
								href="/docs/connect/gcp-cloud-run"
								name="Google Cloud Run"
								icon={gcpLogo}
								alt="Google Cloud Run"
							/>
							<TechLink
								href="/docs/connect/hetzner"
								name="Hetzner"
								icon={hetznerLogo}
								alt="Hetzner"
							/>
							<TechLink
								href="/docs/connect/vm-and-bare-metal"
								name="VM & Bare Metal"
								icon={vmBareMetalLogo}
								alt="VM & Bare Metal"
							/>
							<TechLink
								href="/docs/connect/aws-lambda"
								name="AWS Lambda"
								icon={awsLambdaLogo}
								alt="AWS Lambda"
								status="coming-soon"
							/>
							<TechLink
								href="/docs/connect/supabase"
								name="Supabase"
								icon={supabaseLogo}
								alt="Supabase"
								status="coming-soon"
							/>
							<TechLink
								href="/docs/connect/freestyle"
								name="Freestyle"
								icon={freestyleLogo}
								alt="Freestyle"
								external
								status="coming-soon"
							/>
						</TechSubSection>
					</TechSectionSubsections>
				</TechSectionGroup>

				<TechSectionGroup>
					<TechSectionText
						heading="Works With Your Tools"
						description="Seamlessly integrate Rivet with your favorite frameworks, languages, and tools."
						linkText="Request an integration"
						linkHref="https://github.com/rivet-dev/rivetkit/issues/new"
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
								href="https://github.com/rivet-dev/rivetkit/pull/1172"
								name="Svelte"
								icon={svelteLogo}
								alt="Svelte"
								external
							/>
							<TechLink
								href="https://github.com/rivet-dev/rivetkit/issues/903"
								name="Vue"
								icon={vueLogo}
								alt="Vue"
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
								href="https://github.com/rivet-dev/rivetkit/tree/9a3d850aee45167eadf249fdbae60129bf37e818/examples/ai-agent"
								name="AI SDK"
								icon={vercelLogo}
								alt="AI SDK"
								status="coming-soon"
							/>
						</TechSubSection>

						<TechSubSection title="Sync">
							<TechLink
								href="https://github.com/rivet-dev/rivetkit/issues/908"
								name="LiveStore"
								icon={livestoreLogo}
								alt="LiveStore"
								status="coming-soon"
							/>
							<TechLink
								href="https://github.com/rivet-dev/rivetkit/issues/909"
								name="ZeroSync"
								icon={zerosyncLogo}
								alt="ZeroSync"
								status="help-wanted"
							/>
							<TechLink
								href="https://github.com/rivet-dev/rivetkit/issues/910"
								name="TinyBase"
								icon={tinybaseLogo}
								alt="TinyBase"
								status="help-wanted"
							/>
							<TechLink
								href="https://github.com/rivet-dev/rivetkit/tree/9a3d850aee45167eadf249fdbae60129bf37e818/examples/crdt"
								name="Yjs"
								icon={yjsLogo}
								alt="Yjs"
								status="help-wanted"
							/>
						</TechSubSection>
					</TechSectionSubsections>
				</TechSectionGroup>
			</div>
		</div>
	);
}

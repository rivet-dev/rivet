import type { Metadata } from "next";
import Link from "next/link";
import { Button } from "@rivet-gg/components";
import {
	Icon,
	faCode,
	faLayerGroup,
	faTerminal,
	faDesktop,
	faListCheck,
	faArrowsToCircle,
	faReact,
	faVuejs,
	faAngular,
	faNodeJs,
	faPython,
	faPhp,
	faJava,
	faCss3Alt,
	faHtml5,
	faRust,
	faSwift,
	faJsSquare,
	faGolang,
	faDatabase,
	faDocker,
	faArrowRight,
	faRobot,
	faServer,
	faVectorSquare,
} from "@rivet-gg/icons";
import { CopyCommand } from "./CopyCommand";
import { TutorialsSection } from "./TutorialsSection";
import { CommandCenterSection } from "./CommandCenterSection";
import { CommunitySection } from "./CommunitySection";
import { CtaSection } from "./CtaSection";
import GlobeSvg from "../(img)/globe.svg";
import ActorsSvg from "../(img)/actors.svg";
import ContainerSvg from "../(img)/container.svg";
import WorkflowSvg from "../(img)/workflow.svg";
import DbSvg from "../(img)/db.svg";
import Image from "next/image";

export const metadata: Metadata = {
	title: "Rivet - The Open-Source Serverless Platform",
	description:
		"Easily build & scale AI agents, functions, and more. Open-source & self-hostable.",
};

export default function IndexPage() {
	// an empty div at the top of the page is a workaround for a bug in Next.js that causes the page to jump when the user navigates to it
	// https://github.com/vercel/next.js/discussions/64534
	return (
		<>
			<div />
			<main className="min-h-screen w-full max-w-[1500px] mx-auto md:px-8">
				<Hero />
				<FeaturesGrid />
				<FrameworksSection />
				<TutorialsSection />
				<CommandCenterSection />
				<CommunitySection />
				<CtaSection />
			</main>
		</>
	);
}
// Hero component with title, subtitle, and CTA buttons
const Hero = () => {
	return (
		<div className="relative isolate overflow-hidden pt-14">
			<div className="mx-auto max-w-7xl px-6 pt-24 pb-12 sm:pt-32 sm:pb-16 lg:px-8">
				<div className="mx-auto max-w-4xl text-center">
					<h1 className="text-4xl font-medium tracking-tight text-white sm:text-6xl md:text-6xl">
						The Open-Source
						<br className="hidden sm:block" />
						Serverless Platform
					</h1>
					<p className="mt-8 text-lg leading-8 text-white/70 max-w-2xl mx-auto">
						Easily build & scale{" "}
						<span className="text-white font-medium">
							AI agents
						</span>
						,{" "}
						<span className="text-white font-medium">
							functions
						</span>
						,{" "}
						<span className="text-white font-medium">
							stateful services
						</span>
						, and more.
						<br />
						<span className="text-white/60 font-light">
							Open-source & self-hostable.
						</span>
					</p>
					<div className="mt-10 flex items-center justify-center gap-x-6">
						<Button
							size="lg"
							asChild
							className="px-4 py-3 text-base bg-gradient-to-b from-[#FF5C00] to-[#FF5C00]/90 border border-[#FF5C00]/30 hover:border-[#FF5C00]/60 hover:from-[#E65400] hover:to-[#E65400]/90 transition-all duration-200"
						>
							<Link href="#deploy">
								<span>Deploy in 1 Minute</span>
							</Link>
						</Button>
						<Button
							variant="outline"
							size="lg"
							asChild
							className="px-4 py-3 text-base border-white/10 hover:border-white/30 transition-all duration-200"
						>
							<Link href="#demo">
								<span>Book Demo</span>
							</Link>
						</Button>
					</div>

					<div className="mt-6 text-center">
						<p className="text-white/40 mb-6">or run locally</p>
						<CopyCommand command="docker run rivetgg/rivet:latest" />
					</div>
					<div className="mt-8 inline-flex">
						<Link
							href="/docs/cloudflare-compatibility"
							className="group"
						>
							<div className="text-sm px-4 py-2 bg-[#FF5C00]/5 border border-[#FF5C00]/10 rounded-full inline-flex items-center group-hover:bg-[#FF5C00]/10 group-hover:border-[#FF5C00]/20 transition-all">
								<span className="text-white/70">
									Need on-prem{" "}
									<span className="text-white">
										Cloudflare Workers
									</span>{" "}
									or{" "}
									<span className="text-white">
										Durable Objects
									</span>
									?
								</span>
								<Icon
									icon={faArrowRight}
									className="ml-2 text-xs text-[#FF5C00] group-hover:translate-x-0.5 transition-transform"
								/>
							</div>
						</Link>
					</div>
				</div>
			</div>
		</div>
	);
};

// Feature component for individual features
const Feature = ({
	title,
	description,
	faIcon,
	href,
	useCases,
}: { 
	title: string; 
	description: string; 
	faIcon: any; 
	href: string;
	useCases?: string[];
}) => {
	return (
		<Link href={href} className="block group">
			<div className="rounded-xl bg-[#121212] group-hover:bg-zinc-800/90 border border-white/5 group-hover:border-[white]/20 shadow-sm transition-all duration-200 relative overflow-hidden h-[450px] w-[360px] flex flex-col">
				<div className="px-8 mt-6">
					<div className="flex items-center gap-3 mb-6">
						<Icon
							icon={faIcon}
							className="text-lg text-white/10 group-hover:text-white transition-colors duration-300"
						/>
						<h3 className="text-lg font-normal text-white">
							{title}
						</h3>
					</div>

					<p className="text-white text-sm">{description}</p>
					
					{useCases && useCases.length > 0 && (
						<div className="mt-3 flex flex-wrap gap-x-2 text-xs opacity-0 group-hover:opacity-100 transition-opacity duration-200">
							<span className="text-white/40">Good for:</span> 
							{useCases.map((useCase, index) => (
								<span key={index} className="text-white/70">
									{useCase}{index < useCases.length - 1 ? "," : ""}
								</span>
							))}
						</div>
					)}
				</div>

				<div className="mt-auto">
					{title === "Functions" && (
						<div className="absolute bottom-0 left-0 h-80 w-80 opacity-10 group-hover:opacity-20 transition-opacity duration-200 -ml-8 -mb-36">
							<Image
								src={GlobeSvg}
								alt="Globe"
								fill
								className="object-contain scale-105"
							/>
						</div>
					)}
					{title === "Stateful Actors" && (
						<div className="absolute top-[240px] left-0 h-64 w-64 opacity-10 group-hover:opacity-20 transition-opacity duration-200 -ml-8">
							<Image
								src={ActorsSvg}
								alt="Actors"
								fill
								className="object-contain scale-105"
							/>
						</div>
					)}
					{title === "Sandboxed Containers" && (
						<div className="absolute top-[240px] left-0 h-80 w-80 opacity-10 group-hover:opacity-20 transition-opacity duration-200 -ml-[100px]">
							<Image
								src={ContainerSvg}
								alt="Container"
								fill
								className="object-contain scale-105"
							/>
						</div>
					)}
					{title === "Workflows" && (
						<div className="absolute top-[200px] left-0 h-80 w-80 opacity-10 group-hover:opacity-20 transition-opacity duration-200 -ml-[100px]">
							<Image
								src={WorkflowSvg}
								alt="Workflow"
								fill
								className="object-contain scale-105"
							/>
						</div>
					)}
					{title === "SQLite Databases" && (
						<div className="absolute top-[200px] left-0 h-80 w-80 opacity-10 group-hover:opacity-20 transition-opacity duration-200 -ml-[100px]">
							<Image
								src={DbSvg}
								alt="Database"
								fill
								className="object-contain scale-105"
							/>
						</div>
					)}
					<div className="px-8 pb-8 relative z-10">
						<div className="flex items-center justify-end text-[#505052] opacity-0 group-hover:opacity-100 transition-opacity">
							<Icon
								icon={faArrowRight}
								className="text-xl group-hover:translate-x-0.5 transition-all"
							/>
						</div>
					</div>
				</div>
			</div>
		</Link>
	);
};

// Features grid component
const FeaturesGrid = () => {
	const features = [
		{
			title: "Functions",
			description: "Deploy serverless functions that scale automatically",
			faIcon: faCode,
			href: "/docs/functions",
			useCases: ["APIs", "Edge computing", "Microservices"]
		},
		{
			title: "Stateful Actors",
			description: "Long-running tasks with built-in state persistence & hibernation",
			faIcon: faLayerGroup,
			href: "/docs/stateful-jobs",
			useCases: ["AI agents", "Realtime apps", "Rate limiting"]
		},
		{
			title: "Sandboxed Containers",
			description: "Run long-running tasks in isolated environments",
			faIcon: faServer,
			href: "/docs/stateful-jobs",
			useCases: ["Code interpreters", "Remote desktop", "UGC games"]
		},
		{
			title: "Workflows",
			description: "Orchestrate complex, multi-step processes",
			faIcon: faArrowsToCircle,
			href: "/docs/workflows",
			useCases: ["AI agents", "Business logic", "Data pipelines"]
		},
		{
			title: "SQLite Databases",
			description: "On-demand SQL databases 10x faster than Postgres with vector stores & full text search",
			faIcon: faDatabase,
			href: "/docs/sqlite-databases",
			useCases: ["Agent memory", "Per-tenant databases", "Local-first apps"]
		},
	];

	return (
		<div className="mx-auto w-full px-4 pt-0 pb-16 -mt-8 max-w-[1200px]">
			<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 mt-16 md:mt-20 justify-items-center">
				{features.map((feature, index) => (
					<Feature
						key={index}
						title={feature.title}
						description={feature.description}
						faIcon={feature.faIcon}
						href={feature.href}
						useCases={feature.useCases}
					/>
				))}
			</div>
			<div className="text-center mt-16">
				<p className="text-white/80 text-lg">
					<span className="font-normal text-white">
						Select the products that fit your needs
					</span>{" "}
					— integrated together into a single platform.
				</p>
			</div>
		</div>
	);
};

// Frameworks section
const FrameworksSection = () => {
	const frameworks = [
		{ icon: faReact, name: "React", href: "/docs/frameworks/react" },
		{ icon: faVuejs, name: "Vue", href: "/docs/frameworks/vue" },
		{ icon: faAngular, name: "Angular", href: "/docs/frameworks/angular" },
		{ icon: faNodeJs, name: "Node.js", href: "/docs/frameworks/nodejs" },
		{ icon: faPython, name: "Python", href: "/docs/frameworks/python" },
		{ icon: faPhp, name: "PHP", href: "/docs/frameworks/php" },
		{ icon: faJava, name: "Java", href: "/docs/frameworks/java" },
		{ icon: faRust, name: "Rust", href: "/docs/frameworks/rust" },
		{ icon: faSwift, name: "Swift", href: "/docs/frameworks/swift" },
		{
			icon: faJsSquare,
			name: "JavaScript",
			href: "/docs/frameworks/javascript",
		},
		{ icon: faHtml5, name: "HTML5", href: "/docs/frameworks/html5" },
		{ icon: faCss3Alt, name: "CSS3", href: "/docs/frameworks/css3" },
		{ icon: faGolang, name: "Go", href: "/docs/frameworks/go" },
		{ icon: faDatabase, name: "SQL", href: "/docs/frameworks/sql" },
		{ icon: faDocker, name: "Docker", href: "/docs/frameworks/docker" },
	];

	return (
		<div className="mx-auto max-w-7xl px-6 py-28 lg:py-44 lg:px-8 mt-16">
			<div className="flex flex-col md:flex-row md:items-start">
				<div className="md:w-1/3 mb-8 md:mb-0 md:pr-8">
					<h2 className="text-4xl font-medium tracking-tight text-white text-left">
						Rivet works with any framework
					</h2>
					<p className="mt-4 text-lg text-white/70 text-left">
						Integrate with your existing tech stack or start fresh
						with your preferred tools and languages.
					</p>
				</div>
				<div className="md:w-2/3">
					<div className="grid grid-cols-4 md:grid-cols-5 lg:grid-cols-6 gap-x-6 gap-y-6">
						{frameworks.map((framework, index) => (
							<Link
								key={index}
								href={framework.href}
								className="group"
							>
								<div className="h-16 w-16 mx-auto flex items-center justify-center">
									<Icon
										icon={framework.icon}
										className="text-5xl text-white/30 group-hover:text-white/90 transition-colors duration-200"
										title={framework.name}
									/>
								</div>
							</Link>
						))}
					</div>
				</div>
			</div>
		</div>
	);
};


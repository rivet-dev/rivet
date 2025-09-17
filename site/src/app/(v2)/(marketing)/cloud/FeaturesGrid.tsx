import {
	Icon,
	faActorsBorderless,
	faArrowRight,
	faCode,
	faLayerGroup,
	faServer,
} from "@rivet-gg/icons";
import clsx from "clsx";
import Image from "next/image";
import Link from "next/link";
import ActorsSvg from "./images/actors.svg";
import ContainerSvg from "./images/container.svg";
import DbSvg from "./images/db.svg";
import GlobeSvg from "./images/globe.svg";
import WorkflowSvg from "./images/workflow.svg";

const WT = ({ children }) => <span className="text-white/90">{children}</span>;

// Feature component for individual features
const Feature = ({
	title,
	description,
	faIcon,
	href,
	useCases,
	badge,
}: {
	title: string;
	description: string;
	faIcon: any;
	href: string;
	useCases: string[];
	badge?: string;
}) => {
	const getImageDetails = () => {
		switch (title) {
			case "Functions":
				return {
					src: GlobeSvg,
					alt: "Globe",
					position: "bottom-0 left-0",
					size: "h-80 w-80",
					margin: "-ml-8 -mb-36",
					mobileCss: "", // Globe is already positioned correctly
					desktopCss: "", // Globe is already positioned correctly
				};
			case "Actors":
				return {
					src: ActorsSvg,
					alt: "Actors",
					position: "bottom-0 left-0",
					size: "h-64 w-64",
					margin: "-ml-8 -mb-24",
					mobileCss: "", // Position at bottom for mobile
					desktopCss: "lg:top-[240px] lg:bottom-auto", // Reset to original position for desktop
				};
			case "Containers":
				return {
					src: ContainerSvg,
					alt: "Container",
					position: "bottom-0 left-0",
					size: "h-80 w-80",
					margin: "-ml-[100px] -mb-24",
					mobileCss: "", // Position at bottom for mobile
					desktopCss: "lg:top-[240px] lg:bottom-auto", // Reset to original position for desktop
				};
			case "Workflows":
				return {
					src: WorkflowSvg,
					alt: "Workflow",
					position: "top-[200px] left-0",
					size: "h-80 w-80",
					margin: "-ml-[100px]",
					mobileCss: "",
					desktopCss: "",
				};
			case "SQLite Databases":
				return {
					src: DbSvg,
					alt: "Database",
					position: "top-[200px] left-0",
					size: "h-80 w-80",
					margin: "-ml-[100px]",
					mobileCss: "",
					desktopCss: "",
				};
			default:
				return null;
		}
	};

	const imageDetails = getImageDetails();
	return (
		<Link href={href} className="block group">
			<div className="rounded-xl bg-white/2 border border-white/20 group-hover:border-[white]/40 shadow-sm transition-all duration-200 relative overflow-hidden h-[300px] lg:h-[450px] flex flex-col">
				<div className="px-8 mt-6">
					<div className="flex flex-wrap items-center gap-3 mb-4 text-white text-base">
						<Icon icon={faIcon} />
						<h3>{title}</h3>
						{badge && (
							<span className="px-2 py-0.5 text-xs font-medium rounded-full bg-orange-950/50 text-[#FF5C00]">
								{badge}
							</span>
						)}
					</div>

					<p className="text-white text-base text-white/40 mb-2">
						{description}
					</p>

					{/*<p className="text-white text-base text-white/40">
						{description}<br/>Supports{" "}
						{useCases.map((useCase, index) => (
							<span key={index} className="text-white/90">
								{useCase}
								{index < useCases.length - 1 ? ", " : ""}
							</span>
						))}
						.
					</p>*/}

					<p className="text-white text-base text-white/40">
						Supports{" "}
						{useCases.map((useCase, index) => (
							<span key={index}>
								<span className="text-white/90">{useCase}</span>
								{index < useCases.length - 1 ? ", " : ""}
							</span>
						))}
						.
					</p>

					{/*<div className="mt-3 flex flex-wrap flex-col gap-0.5 text-sm">
						{useCases.map((useCase, index) => (
							<span key={index} className="text-white/90">
								<Icon icon={faCheck} className="mr-1.5" />
								{useCase}
							</span>
						))}
					</div>*/}
				</div>

				<div className="mt-auto">
					{imageDetails && (
						<div
							className={clsx(
								"absolute opacity-10 group-hover:opacity-40 transition-opacity duration-200",
								imageDetails.position,
								imageDetails.size,
								imageDetails.margin,
								imageDetails.mobileCss,
								imageDetails.desktopCss,
							)}
						>
							<Image
								src={imageDetails.src}
								alt={imageDetails.alt}
								fill
								className="object-contain scale-105"
							/>
						</div>
					)}
					<div className="px-8 pb-8 relative z-10">
						<div className="flex items-center justify-end text-white opacity-0 group-hover:opacity-40 transition-opacity">
							<Icon
								icon={faArrowRight}
								className="text-xl -translate-x-1 group-hover:translate-x-0 transition-all"
							/>
						</div>
					</div>
				</div>
			</div>
		</Link>
	);
};

// Features grid component
export const FeaturesGrid = () => {
	const features = [
		{
			//title: "Stateless Functions",
			title: "Rivet Engine",
			description: (
				<>
					<WT>Enterprise-scale orchestration</WT> built on FoundationDB for massive actor distribution and fault tolerance.
				</>
			),
			faIcon: faServer,
			href: "/docs/self-hosting",
			useCases: ["actor orchestration", "multi-region", "fault tolerance"],
			badge: "FoundationDB",
		},
		{
			//title: "Stateful Actors",
			title: "Rivet Actors",
			description: (
				<>
					<WT>Long running tasks</WT> with state persistence,
					hibernation, and realtime that scale infinitely.
				</>
			),
			faIcon: faActorsBorderless,
			href: "/docs/actors",
			useCases: ["AI agents", "realtime apps", "local-first sync"],
			badge: "Open-Source Durable Objects",
		},
		{
			//title: "Sandboxed Containers",
			title: "Platform Integration",
			description: (
				<>
					<WT>Connect to your applications</WT> on Railway, Vercel, and other platforms with seamless integration.
				</>
			),
			faIcon: faLayerGroup,
			href: "/docs/self-hosting/connect-backend",
			useCases: ["Railway", "Vercel", "any platform"],
		},
		//{
		//	title: "Workflows",
		//	description: "Orchestrate complex, multi-step processes",
		//	faIcon: faArrowsToCircle,
		//	href: "/docs/workflows",
		//	useCases: ["AI agents", "Business logic", "Data pipelines"]
		//},
		//{
		//	title: "SQLite Databases",
		//	description: "On-demand SQL databases 10x faster than Postgres with vector stores & full text search",
		//	faIcon: faDatabase,
		//	href: "/docs/sqlite-databases",
		//	useCases: ["Agent memory", "Per-tenant databases", "Local-first apps"]
		//},
	];

	return (
		<div className="mx-auto w-full px-4 pt-0 pb-16 -mt-8 max-w-[1200px] md:px-8">
			<div className="grid grid-cols-1 lg:grid-cols-3 gap-4 mt-16 md:mt-20 lg:justify-items-center">
				{features.map((feature, index) => (
					<Feature
						key={index}
						title={feature.title}
						description={feature.description}
						faIcon={feature.faIcon}
						href={feature.href}
						useCases={feature.useCases}
						badge={feature.badge}
					/>
				))}
			</div>
		</div>
	);
};

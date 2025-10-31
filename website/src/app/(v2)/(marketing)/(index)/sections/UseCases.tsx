"use client";

import { useRef } from "react";
import { useCases } from "@/data/use-cases";
import Link from "next/link";
import { IconWithSpotlight } from "../sections/IconWithSpotlight";

interface UseCaseCardProps {
	title: string;
	description: React.ReactNode;
	href: string;
	className?: string;
	iconPath: string;
	variant?: "default" | "large";
}

function UseCaseCard({
	title,
	description,
	href,
	className = "",
	iconPath,
	variant = "default",
}: UseCaseCardProps) {
	const cardRef = useRef<HTMLAnchorElement>(null);

	const handleMouseMove = (e: React.MouseEvent<HTMLAnchorElement>) => {
		if (!cardRef.current) return;
		const card = cardRef.current;

		// Find the icon container and convert coordinates to icon-relative
		const iconContainer = card.querySelector('.icon-spotlight-container') as HTMLElement;
		if (!iconContainer) return;

		// Get the icon's position relative to viewport
		const iconRect = iconContainer.getBoundingClientRect();

		// Calculate mouse position relative to the icon (not the card)
		const x = ((e.clientX - iconRect.left) / iconRect.width) * 100;
		const y = ((e.clientY - iconRect.top) / iconRect.height) * 100;

		// Set CSS custom properties on the icon container
		iconContainer.style.setProperty('--mouse-x', `${x}%`);
		iconContainer.style.setProperty('--mouse-y', `${y}%`);
	};

	if (variant === "large") {
		return (
			<Link
				ref={cardRef}
				href={href}
				className={`group relative block ${className}`}
				onMouseMove={handleMouseMove}
			>
				<div className="h-full border border-white/20 bg-white/[0.008] hover:border-white/30 hover:bg-white/[0.02] rounded-xl p-6 transition-colors relative overflow-hidden flex flex-row gap-6">
					{/* Gradient overlay on hover */}
					<div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent" />

					{/* Left side: Content + Checkmarks */}
					<div className="relative z-10 flex-1 flex flex-col justify-between">
						<div>
							<h3 className="text-lg font-semibold text-white mb-3">
								{title}
							</h3>
							<p className="text-white/40 text-sm leading-relaxed">
								{description}
							</p>
						</div>

						{/* Checkmarks */}
						<div className="space-y-2 mt-6">
							{["Cloud & on-prem", "Supports realtime", "Works with AI SDK"].map((item) => (
								<div key={item} className="flex items-center gap-2">
									<svg
										className="w-4 h-4 text-white/60"
										fill="none"
										stroke="currentColor"
										viewBox="0 0 24 24"
									>
										<path
											strokeLinecap="round"
											strokeLinejoin="round"
											strokeWidth={2}
											d="M5 13l4 4L19 7"
										/>
									</svg>
									<span className="text-white/60 text-sm">{item}</span>
								</div>
							))}
						</div>
					</div>

					{/* Right side: Icon */}
					<div className="flex-1 flex items-center justify-center relative z-10">
						<IconWithSpotlight iconPath={iconPath} title={title} />
					</div>
				</div>
			</Link>
		);
	}

	return (
		<Link
			ref={cardRef}
			href={href}
			className={`group relative block ${className}`}
			onMouseMove={handleMouseMove}
		>
			<div className="h-full border border-white/20 bg-white/[0.008] hover:border-white/30 hover:bg-white/[0.02] rounded-xl p-6 transition-colors relative overflow-hidden flex flex-col">
				{/* Gradient overlay on hover */}
				<div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent" />

				{/* Content */}
				<div className="relative z-10 mb-6">
					<h3 className="text-lg font-semibold text-white mb-3">
						{title}
					</h3>
					<p className="text-white/40 text-sm leading-relaxed">
						{description}
					</p>
				</div>

				{/* Spotlight icon */}
				<div className="flex-1 flex items-center justify-center relative z-10">
					<IconWithSpotlight iconPath={iconPath} title={title} />
				</div>
			</div>
		</Link>
	);
}

export function UseCases() {
	// Map the use cases we want to display
	const selectedUseCases = [
		useCases.find((uc) => uc.title === "Agent Orchestration & MCP")!, // agent orchestration & mcp
		useCases.find((uc) => uc.title === "Workflows")!, // workflows
		useCases.find((uc) => uc.title === "Multiplayer Apps")!, // multiplayer apps
		useCases.find((uc) => uc.title === "Local-First Sync")!, // local-first sync
		useCases.find((uc) => uc.title === "Background Jobs")!, // background jobs
		useCases.find((uc) => uc.title === "Per-Tenant Databases")!, // per-tenant databases
		useCases.find((uc) => uc.title === "Geo-Distributed Database")!, // geo-distributed database
	];

	// Map use case titles to icon paths
	const getIconPath = (title: string): string => {
		const iconMap: { [key: string]: string } = {
			"Agent Orchestration & MCP": "/use-case-icons/sparkles.svg",
			"Workflows": "/use-case-icons/diagram-next.svg",
			"Multiplayer Apps": "/use-case-icons/file-pen.svg",
			"Local-First Sync": "/use-case-icons/rotate.svg",
			"Background Jobs": "/use-case-icons/gears.svg",
			"Per-Tenant Databases": "/use-case-icons/database.svg",
			"Geo-Distributed Database": "/use-case-icons/globe.svg",
		};
		return iconMap[title] || "";
	};

	// Get highlighted description
	const getHighlightedDescription = (title: string): React.ReactNode => {
		const descriptionMap: { [key: string]: React.ReactNode } = {
			"Agent Orchestration & MCP": (
				<>
					Build <span className="text-white/90">AI agents</span> with Model Context Protocol and persistent state
				</>
			),
			"Workflows": (
				<>
					<span className="text-white/90">Durable multi-step workflows</span> with automatic state management
				</>
			),
			"Multiplayer Apps": (
				<>
					Build <span className="text-white/90">realtime multiplayer</span> applications with authoritative state
				</>
			),
			"Local-First Sync": (
				<>
					<span className="text-white/90">Offline-first</span> applications with server synchronization
				</>
			),
			"Background Jobs": (
				<>
					<span className="text-white/90">Scheduled and recurring jobs</span> without external queue infrastructure
				</>
			),
			"Per-Tenant Databases": (
				<>
					<span className="text-white/90">Isolated data stores</span> for each user with zero-latency access
				</>
			),
			"Geo-Distributed Database": (
				<>
					Store data close to users globally with <span className="text-white/90">automatic edge distribution</span>
				</>
			),
		};
		return descriptionMap[title] || "";
	};

	return (
		<section className="w-full">
			<div className="container relative mx-auto px-6 lg:px-16 xl:px-20 max-w-[1500px]">
				<h2 className="text-2xl sm:text-3xl font-700 text-white mb-8 text-left">
					Actors make it simple to build
				</h2>
				<div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-12 gap-4 md:gap-4 xl:gap-3 2xl:gap-6 auto-rows-[300px]">
					{/* First item - takes 6 columns (half width on medium+) */}
					<UseCaseCard
						title={selectedUseCases[0].title}
						description={getHighlightedDescription(selectedUseCases[0].title)}
						href={selectedUseCases[0].href}
						iconPath={getIconPath(selectedUseCases[0].title)}
						className="sm:col-span-2 md:col-span-6"
						variant="large"
					/>

					{/* Remaining items - 3 columns each (quarter width on medium+) */}
					{selectedUseCases.slice(1).map((useCase) => (
						<UseCaseCard
							key={useCase.href}
							title={useCase.title}
							description={getHighlightedDescription(useCase.title)}
							href={useCase.href}
							iconPath={getIconPath(useCase.title)}
							className="md:col-span-3"
						/>
					))}
				</div>
			</div>
		</section>
	);
}

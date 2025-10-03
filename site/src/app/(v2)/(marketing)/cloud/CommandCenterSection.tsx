"use client";

import imgHub from "@/images/screenshots/rivet-hub.png";
import {
	faCodeBranch,
	faFlask,
	faTerminal,
	faUserGroup,
} from "@rivet-gg/icons";
import Image from "next/image";
import type { FeatureItem } from "./ServerlessLimitationsSection";
import { Feature } from "./components/Feature";

// Command Center section
export const CommandCenterSection = () => {
	const features: FeatureItem[] = [
		{
			name: "Observability & Monitoring",
			icon: faTerminal,
			description:
				"Real-time monitoring out of the box with powerful log querying",
		},
		{
			name: "Preview Environments",
			icon: faFlask,
			description:
				"Test changes in isolated environments before deploying",
		},
		{
			name: "Instant Rollbacks",
			icon: faCodeBranch,
			description: "Safely deploy and revert changes with zero downtime",
		},
		{
			name: "Team Access",
			icon: faUserGroup,
			description: "Collaborate securely with teammates",
		},
	];

	return (
		<div className="w-full px-6 py-40 lg:py-48">
			<div className="relative">
				{/* Screenshot */}
				<div
					className="absolute inset-0 z-[-1] overflow-hidden"
					style={{ perspective: "2000px" }}
				>
					{/* Add a gradient overlay at the container level to fade out the entire element including border */}
					<div
						className="absolute inset-0 z-[1] pointer-events-none"
						style={{
							background:
								"linear-gradient(90deg, hsl(var(--background) / 0) 66%, hsl(var(--background) / 0.95) 85%, hsl(var(--background) / 1) 100%)",
						}}
					/>

					<div
						className="absolute border-2 border-white/10 rounded-md"
						style={{
							transformStyle: "preserve-3d",
							transform:
								"translateX(-11%) scale(1.2) rotateX(38deg) rotateY(19deg) rotateZ(340deg)",
							transformOrigin: "top left",
							top: "210px",
							left: "0",
							right: "0",
							width: "80%",
							maxWidth: "1200px",
							margin: "0 auto",
							aspectRatio: "16/9",
							boxShadow: "0 35px 60px -15px rgba(0, 0, 0, 0.5)",
							zIndex: 0,
						}}
					>
						{/* Hub screenshot with enhanced depth */}
						<div
							className="w-full h-full overflow-hidden"
							style={{
								transformStyle: "preserve-3d",
								backfaceVisibility: "hidden",
								position: "relative",
							}}
						>
							<Image
								src={imgHub}
								alt="Rivet Hub dashboard"
								className="w-full h-auto object-cover object-top"
								fill
								sizes="(max-width: 768px) 100vw, 80vw"
								priority
								quality={90}
							/>
						</div>
					</div>
				</div>

				{/* Content */}
				<div>
					{/* Header */}
					<div className="max-w-7xl mx-auto relative z-10">
						<h2 className="max-w-lg text-4xl font-medium tracking-tight text-white">
							Meet your command center
						</h2>
						<p className="max-w-lg mt-4 text-lg text-white/70">
							Complete visibility and control over your Rivet Actors
						</p>
					</div>

					{/* Spacer */}
					<div className="h-[450px]" />

					{/* Features grid */}
					<div className="pt-16 relative bg-gradient-to-t from-[hsl(var(--background))] via-[hsl(var(--background))] to-transparent z-10">
						<div className="max-w-7xl mx-auto grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6 gap-x-6 gap-y-6">
							{features.map((feature, index) => (
								<Feature key={index} feature={feature} />
							))}
						</div>
					</div>
				</div>
			</div>
		</div>
	);
};

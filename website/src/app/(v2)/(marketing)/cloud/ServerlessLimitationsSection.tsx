"use client";

import {
	faClock,
	faCode,
	faGears,
	faGlobe,
	faNetworkWired,
	faRocketLaunch,
	faTag,
	faTerminal,
} from "@rivet-gg/icons";
import { Feature } from "./components/Feature";

export interface FeatureItem {
	icon: any;
	title?: string;
	name?: string;
	description: string;
}

export const ServerlessLimitationsSection = () => {
	const features: FeatureItem[] = [
		{
			icon: faClock,
			title: "Unlimited Execution Time",
			description:
				"No arbitrary timeout limits for your long-running processes",
		},
		{
			icon: faRocketLaunch,
			title: "No Cold Starts",
			description: "Instant availability without delay",
		},
		{
			icon: faCode,
			title: "Docker Compatibility",
			description:
				"Works with anything that runs in Docker - no proprietary runtimes",
		},
		{
			icon: faTerminal,
			title: "Local Development",
			description:
				"Run a full Rivet instance with one command or in Docker Compose",
		},
		{
			icon: faNetworkWired,
			title: "Supports Any Protocol",
			description: "HTTP, WebSocket, TCP, and UDP support built-in",
		},
		{
			icon: faGlobe,
			title: "Control Where Code Runs",
			description:
				"Deploy near users or data centers for optimal performance",
		},
		{
			icon: faGears,
			title: "Customizable Rollouts",
			description:
				"Control exactly how and when your services are updated",
		},
		{
			icon: faTag,
			title: "Tagged Resources",
			description:
				"Organize and manage resources with powerful tagging system",
		},
	];

	return (
		<div className="mx-auto max-w-7xl py-32 lg:py-40">
			<div className="text-center mb-12">
				<h2 className="text-4xl font-medium tracking-tight text-white">
					Rivet without limitations
				</h2>
				<p className="mt-4 text-lg text-white/70">
					All the benefits of Rivet on a managed cloud with custom features
				</p>
			</div>

			<div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6">
				{features.map((feature, index) => (
					<Feature key={index} feature={feature} />
				))}
			</div>
		</div>
	);
};

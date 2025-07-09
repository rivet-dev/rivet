"use client";

import Link from "next/link";
import {
	Icon,
	faCloud,
	faServer,
	faGithub,
	faArrowRight,
} from "@rivet-gg/icons";

interface DeploymentOption {
	icon: any;
	title: string;
	description: string;
	subdescription?: string;
	buttonText: string;
	buttonHref: string;
}

export const RivetCloudSection = () => {
	const deploymentOptions: DeploymentOption[] = [
		{
			icon: faCloud,
			title: "Rivet Cloud",
			description:
				"Fully-managed serverless platform with enterprise support. Let us handle the infrastructure while you focus on your code.",
			buttonText: "See Pricing",
			buttonHref: "/pricing",
		},
		{
			icon: faServer,
			title: "Bring Your Own Cloud",
			description:
				"Specify your cloud of choice or provide your own API keys to deploy Rivet within your own cloud. Maintain control of where your where your backend runs while Rivet handles the maintenance.",
			buttonText: "Contact Us",
			buttonHref: "/sales",
		},
		{
			icon: faGithub,
			title: "Self-Hosted",
			description:
				"Run Rivet on your own infrastructure. Completely open-source and free to use.",
			buttonText: "View on GitHub",
			buttonHref: "https://github.com/rivet-gg/rivet",
		},
	];

	return (
		<div className="mx-auto max-w-7xl px-6 py-32 lg:py-40">
			<div className="text-center mb-12">
				<h2 className="text-4xl font-medium tracking-tight text-white">
					Rivet on Any Cloud
				</h2>
				<p className="mt-4 text-lg text-white/70">
					Choose the deployment model that works best for your
					organization
				</p>
			</div>

			<div className="overflow-hidden rounded-lg border border-zinc-700">
				<div className="grid grid-cols-1 md:grid-cols-3 divide-y md:divide-y-0 md:divide-x divide-zinc-700">
					{deploymentOptions.map((option, index) => (
						<div
							key={index}
							className="bg-white/2 px-10 py-8 md:p-12 flex flex-col"
						>
							<div className="flex-1">
								<div className="mb-5">
									<Icon
										icon={option.icon}
										className="text-2xl text-white/90"
									/>
								</div>

								<h3 className="text-xl font-medium text-white mb-4">
									{option.title}
								</h3>

								<p className="text-white/60">
									{option.description}
								</p>

								{option.subdescription && (
									<p className="text-white/40 mt-2">
										{option.subdescription}
									</p>
								)}
							</div>

							<div className="h-6" />

							<div className="mt-auto">
								<Link
									href={option.buttonHref}
									className="text-white inline-flex items-center group hover:text-white/80 transition-colors"
								>
									<span className="font-medium">
										{option.buttonText}
									</span>
									<Icon
										icon={faArrowRight}
										className="ml-2 text-xs group-hover:translate-x-0.5 transition-transform"
									/>
								</Link>
							</div>
						</div>
					))}
				</div>
			</div>
		</div>
	);
};

"use client";

import { Icon } from "@rivet-gg/icons";
import { FeatureItem } from "../ServerlessLimitationsSection";

interface FeatureProps {
	feature: FeatureItem;
}

export function Feature({ feature }: FeatureProps) {
	const title = feature.title || feature.name;
	
	return (
		<div className="p-5 text-md">
			<div className="flex items-center gap-3 mb-3 text-white/90">
				<Icon icon={feature.icon} />
				<span className="font-medium text-white">{title}</span>
			</div>
			<p className="text-white/60">{feature.description}</p>
		</div>
	);
}

interface FeatureGridProps {
	features: FeatureItem[];
	className?: string;
}

export function FeatureGrid({ features, className }: FeatureGridProps) {
	return (
		<div className={className}>
			{features.map((feature, index) => (
				<Feature key={index} feature={feature} />
			))}
		</div>
	);
}

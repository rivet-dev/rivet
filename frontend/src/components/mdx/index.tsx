import {
	faExclamationTriangle,
	faInfoCircle,
	faLightbulbOn,
	Icon,
} from "@rivet-gg/icons";

import type { ComponentProps, PropsWithChildren, ReactNode } from "react";
import { CardGroup, CtaCard } from "../cta-card";
import { cn } from "../lib/utils";
import { WithTooltip } from "../ui/tooltip";
import { Link } from "../ui/typography";

// Custom CalloutBase component for dark backgrounds
const CalloutBase = ({
	icon,
	children,
	iconColor,
	bgColor,
	borderColor,
	textColor,
}: {
	icon: any;
	children: ReactNode;
	iconColor: string;
	bgColor: string;
	borderColor: string;
	textColor: string;
}) => {
	return (
		<div
			className={cn(
				"relative w-full rounded-xl border p-4 my-4 flex gap-3",
				bgColor,
				borderColor,
			)}
		>
			<div className="flex-shrink-0 pt-0.5">
				<Icon icon={icon} className={cn("w-5 h-5", iconColor)} />
			</div>
			<div
				className={cn(
					"flex-1 prose-invert prose max-w-full w-full",
					textColor,
				)}
			>
				{children}
			</div>
		</div>
	);
};

// biome-ignore lint/a11y/useAltText: dev's responsibility
export const Image = (props: ComponentProps<"img">) => <img {...props} />;

export const Card = ({
	href,
	...props
}: ComponentProps<typeof CtaCard> & { href?: string }) => {
	if (href) {
		return (
			<Link href={href} className="h-full">
				<CtaCard className="h-full" {...props} />
			</Link>
		);
	}
};

export { CardGroup };

export const Warning = ({ children }: { children: ReactNode }) => {
	return (
		<CalloutBase
			icon={faExclamationTriangle}
			iconColor="text-yellow-400"
			bgColor="bg-yellow-950/20"
			borderColor="border-yellow-700/50"
			textColor="text-yellow-100"
		>
			{children}
		</CalloutBase>
	);
};

export const Tip = ({ children }: { children: ReactNode }) => {
	return (
		<CalloutBase
			icon={faLightbulbOn}
			iconColor="text-green-400"
			bgColor="bg-green-950/20"
			borderColor="border-green-700/50"
			textColor="text-green-100"
		>
			{children}
		</CalloutBase>
	);
};

export const Info = ({ children }: { children: ReactNode }) => {
	return (
		<CalloutBase
			icon={faInfoCircle}
			iconColor="text-blue-400"
			bgColor="bg-blue-950/20"
			borderColor="border-blue-700/50"
			textColor="text-blue-100"
		>
			{children}
		</CalloutBase>
	);
};

export const Note = ({ children }: { children: ReactNode }) => {
	return (
		<CalloutBase
			icon={faInfoCircle}
			iconColor="text-gray-400"
			bgColor="bg-gray-950/20"
			borderColor="border-gray-700/50"
			textColor="text-gray-100"
		>
			{children}
		</CalloutBase>
	);
};

export { Step, Steps } from "../steps";
export * from "./code";
export * from "./tabs";

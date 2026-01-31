import { Slot, Slottable } from "@radix-ui/react-slot";
import { faChevronRight, Icon, type IconProp } from "@rivet-gg/icons";
import { type ComponentProps, cloneElement } from "react";
import { cn } from "./lib/utils";

interface ExternalLinkCardProps {
	href: string;
	icon: IconProp;
	title: string;
	description?: string;
}

export function ExternalLinkCard({
	href,
	icon,
	title,
	description = "Opens in a new tab",
}: ExternalLinkCardProps) {
	return (
		<a href={href} target="_blank" rel="noreferrer" className="block">
			<div className="border rounded-lg p-4 hover:border-primary transition-colors cursor-pointer">
				<Card icon={icon} title={title} description={description} />
			</div>
		</a>
	);
}

export function ButtonCard({
	icon,
	title,
	description = "Opens in a new tab",
	className,
	...props
}: CardProps & ComponentProps<"button">) {
	return (
		<button
			{...props}
			className={cn(
				"border rounded-lg p-4 hover:border-primary transition-colors cursor-pointer",
				className,
			)}
		>
			<Card icon={icon} title={title} description={description} />
		</button>
	);
}

export function ExternalCard({
	icon,
	title,
	description = "Opens in a new tab",
}: Omit<ExternalLinkCardProps, "href">) {
	return (
		<div className="border rounded-lg p-4 hover:border-primary transition-colors cursor-pointer">
			<Card icon={icon} title={title} description={description} />
		</div>
	);
}

interface CardProps {
	icon: IconProp;
	title: string;
	description: string;
}

const Card = ({ icon, title, description }: CardProps) => {
	return (
		<div className="flex items-center justify-between gap-3">
			<div className="flex items-center gap-3">
				<Icon icon={icon} className="text-2xl" />
				<div>
					<div className="font-medium">{title}</div>
					<div className="text-sm text-muted-foreground">
						{description}
					</div>
				</div>
			</div>
			<Icon icon={faChevronRight} className="text-muted-foreground" />
		</div>
	);
};

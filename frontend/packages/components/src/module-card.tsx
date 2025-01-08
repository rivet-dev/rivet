"use client";
import type { IconProp } from "@fortawesome/fontawesome-svg-core";
import { faExternalLink } from "@rivet-gg/icons";
import { Icon } from "@rivet-gg/icons";
import { motion } from "framer-motion";
import { Suspense, lazy } from "react";
import { cn } from "./lib/utils";
import { Card, CardDescription, CardHeader, CardTitle } from "./ui/card";
import {
	Sheet,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
	SheetTrigger,
} from "./ui/sheet";
import { Link, Text } from "./ui/typography";

const ModuleIcon = lazy(async () => ({
	default: (await import("./module-icon")).ModuleIcon,
}));

const animationProps = {
	initial: { opacity: 0 },
	animate: { opacity: 1 },
	exit: { opacity: 0 },
};

interface ModuleCardProps {
	id: string;
	name: string;
	className?: string;
	description: string;
	status: string;
	icon: string;
	layoutAnimation?: boolean;
	onClick?: () => void;
}

export function ModuleCard({
	id,
	name,
	icon,
	description,
	className,
	layoutAnimation = true,
	onClick,
}: ModuleCardProps) {
	return (
		<Card
			className={cn(
				"text-left hover:border-primary transition-colors",
				className,
			)}
			asChild
		>
			<motion.button
				type="button"
				className="h-full flex"
				onClick={onClick}
				{...(layoutAnimation
					? { ...animationProps, layout: "position", layoutId: id }
					: {})}
			>
				<CardHeader className="max-w-full">
					<CardTitle>
						<Suspense fallback={null}>
							<ModuleIcon
								className="text-primary mb-3 block"
								icon={icon as IconProp}
							/>
						</Suspense>
						{name}
					</CardTitle>
					<CardDescription className="break-words">
						{description}
					</CardDescription>
				</CardHeader>
			</motion.button>
		</Card>
	);
}

export function DocumentedModuleCard(props: ModuleCardProps) {
	const { name, id } = props;

	return (
		<Sheet>
			<SheetTrigger asChild>
				<ModuleCard {...props} />
			</SheetTrigger>
			<SheetContent className="sm:max-w-[500px]">
				<SheetHeader>
					<SheetTitle>{name}</SheetTitle>
					<Text className="text-xs">
						<Link
							href={`https://rivet.gg/modules/${id}?utm_source=hub`}
							target="_blank"
							rel="noopener noreferrer"
						>
							Open in New Tab <Icon icon={faExternalLink} />
						</Link>
					</Text>
					<SheetDescription className="-mx-6" asChild>
						<div>
							<iframe
								className="w-full h-screen border-t"
								src={`https://rivet.gg/modules/${id}?embed=true`}
								title={name}
							/>
						</div>
					</SheetDescription>
				</SheetHeader>
			</SheetContent>
		</Sheet>
	);
}

import { Icon, faTs, faFunction, faServer, faActors } from "@rivet-gg/icons";
import { motion } from "framer-motion";
import type { ComponentProps } from "react";
import { Button } from "../ui/button";
import { DocsSheet } from "../docs-sheet";
import { cn } from "../lib/utils";
import { CardHeader, CardTitle, CardContent, Card } from "../ui/card";

export function ActorsResources() {
	return (
		<>
			<Card className="max-w-2xl mx-auto w-full my-6">
				<CardHeader>
					<CardTitle>Getting Started</CardTitle>
				</CardHeader>
				<CardContent className="grid md:grid-cols-2 gap-4">
					<ExampleLink
						href="docs/actors"
						title="Rivet Actors"
						size="md"
						icon={faActors}
					/>
					<ExampleLink
						href="docs/containers"
						title="Rivet Containers"
						size="md"
						icon={faServer}
					/>
					<ExampleLink
						href="docs/functions"
						title="Rivet Functions"
						size="md"
						icon={faTs}
					/>
				</CardContent>
			</Card>
		</>
	);
}

const linkVariants = {
	hidden: {
		opacity: 0,
	},
	show: {
		opacity: 1,
	},
};

interface ExampleLinkProps {
	title: string;
	description?: string;
	icon: ComponentProps<typeof Icon>["icon"];
	href: string;
	size?: "sm" | "md" | "lg";
}

function ExampleLink({
	title,
	description,
	icon,
	href,
	size = "lg",
}: ExampleLinkProps) {
	return (
		<DocsSheet path={href} title={title}>
			<Button variant="outline" asChild className="py-4 px-3">
				<motion.button
					key={title}
					type="button"
					variants={linkVariants}
					className={cn(
						"grid grid-cols-[40px,1fr] items-center h-auto max-h-none",
						{
							"grid-cols-[min-content,1fr]": size === "md",
							"grid-cols-[40px,1fr]": size === "lg",
						},
					)}
				>
					<div className="items-center justify-center flex">
						<Icon
							className={cn({
								"text-xl": size === "md",
								"text-3xl": size === "lg",
							})}
							icon={icon}
						/>
					</div>
					<div className="ml-3 flex gap-0.5 flex-col text-left">
						<span className="font-semibold">{title}</span>
						{description ? (
							<span className="text-muted-foreground">
								{description}{" "}
							</span>
						) : null}
					</div>
				</motion.button>
			</Button>
		</DocsSheet>
	);
}

import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	DocsSheet,
	cn,
} from "@rivet-gg/components";
import {
	Icon,
	faActors,
	faArrowProgress,
	faChevronDoubleDown,
	faChevronRight,
	faCircleNodes,
	faDiagramNext,
	faUpRightAndDownLeftFromCenter,
} from "@rivet-gg/icons";
import { motion, useMotionValueEvent, useScroll } from "framer-motion";
import { type ComponentProps, useState } from "react";

import * as components from "@rivet-gg/components/mdx";
import Setup from "../../../../../site/src/content/docs/setup.mdx";

const containerVariants = {
	hidden: {
		opacity: 0,
	},
	show: {
		opacity: 1,
		transition: {
			staggerChildren: 0.2,
		},
	},
};

export function GetStarted() {
	const { scrollY } = useScroll();
	const [isScrolled, setScrolled] = useState(false);

	useMotionValueEvent(scrollY, "change", (current) => {
		const isScrolledEnough = current > 100;
		setScrolled(isScrolledEnough);
	});
	return (
		<>
			<motion.div
				initial={{ y: 0, x: "-50%" }}
				animate={{ y: isScrolled ? 100 : 0, x: "-50%" }}
				className="fixed bottom-4 left-1/2 -translate-x-1/2 z-50"
			>
				<Button
					onClick={() => {
						document
							.getElementById("build-with-rivet")
							?.scrollIntoView({ behavior: "smooth" });
					}}
					endIcon={<Icon icon={faChevronDoubleDown} />}
				>
					See more
				</Button>
			</motion.div>
			<Card className="max-w-2xl mx-auto w-full my-6 ">
				<CardContent>
					<div className="prose-invert prose mt-8">
						<Setup components={components} />
					</div>
				</CardContent>
			</Card>
			{/* <Card
				id="examples"
				asChild
				className="max-w-2xl w-full mx-auto my-6"
			>
				<motion.div>
					<CardHeader>
						<CardTitle>Examples</CardTitle>
						<CardDescription>
							Dive into our example projects to get a feel for how
							Rivet can help you build your next project.
						</CardDescription>
					</CardHeader>
					<CardContent>
						<motion.div
							variants={containerVariants}
							initial="hidden"
							whileInView="show"
							viewport={{ once: true }}
							className="grid md:grid-cols-3 gap-4"
						>
							<ExampleLink
								href="examples"
								title="Multiplayer Tools"
								size="md"
								icon={faArrowPointer}
							/>
							<ExampleLink
								href="examples"
								title="Local-First Apps"
								size="md"
								icon={faWifiSlash}
							/>
							<ExampleLink
								href="examples"
								title="AI Agents"
								size="md"
								icon={faSparkles}
							/>
							<ExampleLink
								href="examples"
								title="Run User Code"
								size="md"
								icon={faCode}
							/>
							<ExampleLink
								href="examples"
								title="Game Servers"
								size="md"
								icon={faGamepadAlt}
							/>
						</motion.div>
					</CardContent>
				</motion.div>
			</Card> */}
			<Card
				asChild
				className="max-w-2xl w-full mx-auto my-6"
				id="build-with-rivet"
			>
				<motion.div>
					<CardHeader>
						<CardTitle>Build with Rivet</CardTitle>
						<CardDescription>
							Explore the possibilities of what you can build with
							Rivet.
						</CardDescription>
					</CardHeader>
					<CardContent>
						<motion.div
							variants={containerVariants}
							initial="hidden"
							whileInView="show"
							viewport={{ once: true }}
							className="grid md:grid-cols-2 gap-4"
						>
							<ExampleLink
								href="docs"
								title="What are Actors?"
								size="md"
								icon={faActors}
							/>
							<ExampleLink
								href="docs/rpc"
								title="RPC"
								size="md"
								icon={faArrowProgress}
							/>
							<ExampleLink
								href="docs/state"
								size="md"
								title="State"
								icon={faDiagramNext}
							/>
							<ExampleLink
								href="docs/scaling"
								title="Scaling & Concurrency"
								size="md"
								icon={faUpRightAndDownLeftFromCenter}
							/>
							<ExampleLink
								href="docs/edge"
								size="md"
								title="Edge Networking"
								icon={faCircleNodes}
							/>
						</motion.div>

						<div className="flex items-center justify-center">
							<Button
								asChild
								variant="link"
								endIcon={<Icon icon={faChevronRight} />}
								className="mt-4 mx-auto"
							>
								<motion.a
									variants={linkVariants}
									href="https://rivet.gg/docs?utm_source=hub"
									target="_blank"
									rel="noreferrer"
								>
									More
								</motion.a>
							</Button>
						</div>
					</CardContent>
				</motion.div>
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

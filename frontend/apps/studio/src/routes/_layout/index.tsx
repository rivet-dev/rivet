import { Actors } from "@/components/actors";
import {
	connectionEffect,
	connectionStateAtom,
	initiallyConnectedAtom,
} from "@/stores/manager";
import {
	Button,
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
	CodeFrame,
	CodeGroup,
	CodeSource,
	DocsSheet,
	H1,
	Link,
	Strong,
} from "@rivet-gg/components";
import {
	ActorsListFiltersSchema,
	currentActorIdAtom,
} from "@rivet-gg/components/actors";
import {
	Icon,
	faChrome,
	faBrave,
	faReact,
	faRust,
	faSafari,
	faTs,
} from "@rivet-gg/icons";
import { createFileRoute } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { AnimatePresence, motion } from "framer-motion";
import { useAtom, useAtomValue, useSetAtom } from "jotai";
import { useEffect } from "react";
import { z } from "zod";
// @ts-expect-error types are missing
import devNpm, { source as devNpmSource } from "../../content/dev-npm.sh?shiki";
import devYarn, {
	source as devYarnSource,
	// @ts-expect-error types are missing
} from "../../content/dev-yarn.sh?shiki";
import devPnpm, {
	source as devPnpmSource,
	// @ts-expect-error types are missing
} from "../../content/dev-pnpm.sh?shiki";

import devBun, {
	source as devBunSource,
	// @ts-expect-error types are missing
} from "../../content/dev-bun.sh?shiki";

export const Route = createFileRoute("/_layout/")({
	component: RouteComponent,
	validateSearch: zodValidator(
		z
			.object({
				actorId: z.string().optional(),
				tab: z.string().optional(),
			})
			.merge(ActorsListFiltersSchema),
	),
});

function RouteComponent() {
	useAtom(connectionEffect);

	const isInitiallyConnected = useAtomValue(initiallyConnectedAtom);
	const status = useAtomValue(connectionStateAtom);

	const { actorId } = Route.useSearch();

	const setCurrentActorId = useSetAtom(currentActorIdAtom);

	useEffect(() => {
		setCurrentActorId(actorId);
	}, [actorId, setCurrentActorId]);

	return (
		<AnimatePresence initial={false}>
			{status === "disconnected" && !isInitiallyConnected ? (
				<motion.div
					initial={{ opacity: 0, scale: 0.95 }}
					animate={{ opacity: 1, scale: 1 }}
					exit={{ opacity: 0, scale: 0.95 }}
					className="size-full flex items-center justify-center-safe flex-col overflow-auto"
				>
					<H1 className="mt-4">Rivet Studio</H1>
					<Card className="max-w-md w-full mb-6 mt-8">
						<CardHeader>
							<CardTitle>Getting Started</CardTitle>
						</CardHeader>
						<CardContent>
							<p>
								Get started with one of our quick start guides:
							</p>
							<div className="flex-1 flex flex-col gap-2 mt-4">
								<div className="flex flex-row justify-stretch items-center gap-2">
									<DocsSheet
										path="https://actorcore.org/frameworks/react"
										title="React Quick Start"
									>
										<Button
											className="flex-1"
											variant="outline"
											startIcon={<Icon icon={faReact} />}
										>
											React
										</Button>
									</DocsSheet>
									<DocsSheet
										path="https://actorcore.org/clients/javascript"
										title="TypeScript Quick Start"
									>
										<Button
											className="flex-1"
											variant="outline"
											startIcon={<Icon icon={faTs} />}
										>
											TypeScript
										</Button>
									</DocsSheet>
									<DocsSheet
										path="https://actorcore.org/clients/rust"
										title="Rust Quick Start"
									>
										<Button
											className="flex-1"
											variant="outline"
											startIcon={<Icon icon={faRust} />}
										>
											Rust
										</Button>
									</DocsSheet>
								</div>
							</div>
						</CardContent>
					</Card>
					<Card className="max-w-md w-full my-6">
						<CardHeader>
							<CardTitle>Connect to Project</CardTitle>
						</CardHeader>
						<CardContent>
							<p>
								Connect Rivet Studio to your ActorCore project,
								using the following command:
							</p>

							<CodeGroup>
								<CodeFrame
									title="npm"
									language="bash"
									code={devNpmSource}
								>
									<CodeSource>{devNpm}</CodeSource>
								</CodeFrame>
								<CodeFrame
									title="pnpm"
									language="bash"
									code={devPnpmSource}
								>
									<CodeSource>{devPnpm}</CodeSource>
								</CodeFrame>
								<CodeFrame
									title="bun"
									language="bash"
									code={devBunSource}
								>
									<CodeSource>{devBun}</CodeSource>
								</CodeFrame>
								<CodeFrame
									title="yarn"
									language="bash"
									code={devYarnSource}
								>
									<CodeSource>{devYarn}</CodeSource>
								</CodeFrame>
							</CodeGroup>
						</CardContent>
					</Card>

					<Card className="max-w-md w-full my-6">
						<CardHeader>
							<CardTitle>Having trouble connecting?</CardTitle>
						</CardHeader>
						<CardContent>
							<p>
								Rivet Studio works best in{" "}
								<Strong>
									<Icon icon={faChrome} /> Chrome
								</Strong>
								. Some browsers like{" "}
								<Strong>
									<Icon icon={faSafari} /> Safari
								</Strong>{" "}
								and{" "}
								<Strong>
									<Icon icon={faBrave} /> Brave
								</Strong>{" "}
								block access to localhost by default.
							</p>
						</CardContent>
						<CardFooter>
							<p className="text-muted-foreground text-sm">
								Having issues? Join the{" "}
								<Link
									href="https://rivet.gg/discord"
									target="_blank"
									rel="noopener noreferrer"
								>
									Rivet Discord
								</Link>{" "}
								or{" "}
								<Link
									href="https://github.com/rivet-gg/rivet/issues"
									target="_blank"
									rel="noopener noreferrer"
								>
									file a GitHub Issue
								</Link>
								.
							</p>
						</CardFooter>
					</Card>
				</motion.div>
			) : (
				<Actors actorId={actorId} />
			)}
		</AnimatePresence>
	);
}

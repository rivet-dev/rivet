import type { Rivet } from "@rivet-gg/api";
import { H1, Lead, Page } from "@rivet-gg/components";
import { LayoutGroup, motion } from "framer-motion";
import { Intro } from "../intro";
import { OnboardingBackground } from "../onboarding-background";

interface OnboardingProps {
	initialProjectName?: string;
	initialStep?: number;
	onFinish?: (project: Rivet.game.GameSummary) => Promise<void> | void;
}

export function Onboarding({
	initialProjectName,
	initialStep,
	onFinish,
}: OnboardingProps) {
	return (
		<>
			<OnboardingBackground />
			<Page className="relative h-full flex flex-col items-center justify-center">
				<div className="">
					<H1 asChild className="text-center mb-2">
						<motion.h1
							initial={{ opacity: 0 }}
							animate={{ opacity: 1 }}
						>
							Get Started
						</motion.h1>
					</H1>
					<Lead asChild className="mx-auto text-center max-w-md mb-8">
						<motion.p
							initial={{ opacity: 0 }}
							animate={{ opacity: 1, transition: { delay: 0.1 } }}
						>
							Your system to build robust, fast, and scalable
							applications on the edge.
						</motion.p>
					</Lead>

					<motion.div
						initial={{ opacity: 0 }}
						animate={{ opacity: 1, transition: { delay: 0.5 } }}
					>
						<LayoutGroup>
							<Intro
								onFinish={onFinish}
								initialProjectName={initialProjectName}
								initialStep={initialStep}
							/>
						</LayoutGroup>
					</motion.div>
				</div>
			</Page>
		</>
	);
}

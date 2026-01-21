import { faCode, faSparkles, faStamp } from "@rivet-gg/icons";
import { motion } from "framer-motion";
import { Content } from "@/app/layout";
import { MotionLink } from "@/app/motion-link";
import { ExternalCard, H1 } from "@/components";
import { TEST_IDS } from "@/utils/test-ids";
import { OnboardingFooter } from "./footer";

export function PathSelection() {
	return (
		<Content className="flex flex-col items-center justify-safe-center">
			<div className="flex flex-col mx-auto flex-1 w-full px-6 items-center justify-center">
				<H1 className="mt-8 text-center">Get started with Rivet</H1>
				<p className="text-center text-muted-foreground max-w-2xl mx-auto mt-2">
					Choose your preferred method to set up your Rivet project.
				</p>
				<motion.div
					className="flex flex-col gap-6 mt-8"
					initial="initial"
					animate="show"
					data-testid={TEST_IDS.Onboarding.PathSelection}
					variants={{
						initial: {},
						show: {
							transition: {
								staggerChildren: 0.05,
							},
						},
					}}
				>
					<MotionLink
						to="."
						search={{ flow: "agent", noTemplate: true }}
						className="text-left"
						data-testid={TEST_IDS.Onboarding.PathSelectionAgent}
						variants={{
							initial: { opacity: 0, y: 10 },
							show: { opacity: 1, y: 0 },
						}}
					>
						<ExternalCard
							icon={faSparkles}
							title="Use Coding Agent"
							description="Let your Coding Agent create and configure your project for you"
						/>
					</MotionLink>
					<MotionLink
						to="."
						search={{ flow: "template" }}
						className="text-left"
						data-testid={TEST_IDS.Onboarding.PathSelectionTemplate}
						variants={{
							initial: { opacity: 0, y: 10 },
							show: { opacity: 1, y: 0 },
						}}
					>
						<ExternalCard
							icon={faStamp}
							title="Start from template"
							description="Quickly set up a project using a pre-built template"
						/>
					</MotionLink>
					<MotionLink
						to="."
						search={{ flow: "manual" }}
						className="text-left"
						data-testid={TEST_IDS.Onboarding.PathSelectionManual}
						variants={{
							initial: { opacity: 0, y: 10 },
							show: { opacity: 1, y: 0 },
						}}
					>
						<ExternalCard
							icon={faCode}
							title="Integrate manually"
							description="Manually set up your Rivet project step-by-step"
						/>
					</MotionLink>
				</motion.div>
			</div>
			<OnboardingFooter />
		</Content>
	);
}

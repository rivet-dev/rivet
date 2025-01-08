import { Intro } from "@/components/intro";
import { OnboardingBackground } from "@/components/onboarding-background";
import { guardOssNewbie } from "@/lib/guards";
import { H1, Lead, Page } from "@rivet-gg/components";
import { createFileRoute } from "@tanstack/react-router";
import { zodSearchValidator } from "@tanstack/router-zod-adapter";
import { LayoutGroup, motion } from "framer-motion";
import { z } from "zod";

function IndexRoute() {
	const { initialStep, project_name: projectName } = Route.useSearch();
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
								initialProjectName={projectName}
								initialStep={initialStep}
							/>
						</LayoutGroup>
					</motion.div>
				</div>
			</Page>
		</>
	);
}

const searchSchema = z.object({
	newbie: z.coerce.boolean().optional(),
	initialStep: z.coerce.number().optional(),
	project_name: z.coerce.string().optional(),
});

export const Route = createFileRoute("/_authenticated/_layout/")({
	validateSearch: zodSearchValidator(searchSchema),
	component: IndexRoute,
	staticData: {
		layout: "onboarding",
	},
	beforeLoad: ({ search, context: { queryClient, auth } }) => {
		if (search.newbie === true) {
			return;
		}
		return guardOssNewbie({ queryClient, auth });
	},
	shouldReload: true,
});

import { usePageLayout } from "@/lib/compute-page-layout";
import { useRouterState } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { useState } from "react";

export function OnboardingBackground() {
	const layout = usePageLayout();
	const [isLoaded, setIsLoaded] = useState(false);
	const isRouteLoaded = useRouterState({
		select(state) {
			return state.loadedAt;
		},
	});

	if (layout !== "onboarding") {
		return null;
	}

	return (
		<motion.video
			autoPlay
			initial={{ opacity: 0 }}
			animate={{
				opacity: isLoaded && isRouteLoaded ? 1 : 0,
			}}
			onLoadedData={() => {
				setIsLoaded(true);
			}}
			muted
			loop
			className="fixed top-0 left-0 w-full h-full object-cover object-center"
		>
			<source
				src="https://assets2.rivet.gg/background/floating-blobs.mp4"
				type="video/mp4"
			/>
		</motion.video>
	);
}

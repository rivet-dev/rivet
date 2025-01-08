import { featuredModulesQueryOptions } from "@/domains/project/queries";
import { DocumentedModuleCard, Grid, H2 } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { motion } from "framer-motion";
import type { ReactNode } from "react";

interface FeaturesModulesGridProps {
	footer?: ReactNode;
}

export function FeaturesModulesGrid({ footer }: FeaturesModulesGridProps) {
	const { data } = useSuspenseQuery(featuredModulesQueryOptions());

	return (
		<motion.section
			className="mt-12"
			initial={{ opacity: 0 }}
			animate={{ opacity: 1 }}
			exit={{ opacity: 0 }}
		>
			<H2 className="my-4 text-base">Extend your project with modules</H2>

			<Grid
				columns={{ initial: "1", sm: "3" }}
				gap="4"
				items="start"
				className="my-4"
			>
				{data.map(({ id, module }) => (
					<DocumentedModuleCard
						key={id}
						id={id}
						layoutAnimation={false}
						{...module.config}
					/>
				))}
			</Grid>
			{footer ? <div className="text-right">{footer}</div> : null}
		</motion.section>
	);
}

export default FeaturesModulesGrid;

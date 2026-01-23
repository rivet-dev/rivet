import { faChevronLeft, Icon } from "@rivet-gg/icons";
import { LayoutGroup, motion } from "framer-motion";
import type { ComponentProps, ReactNode } from "react";
import { Templates } from "@/app/templates";
import { OnboardingFooter } from "./onboarding/footer";
import { Button } from "./ui/button";
import { H1 } from "./ui/typography";

export function TemplatesList({
	back,
	...props
}: ComponentProps<typeof Templates> & {
	back?: ReactNode;
}) {
	return (
		<div className="h-screen flex flex-col justify-safe-center">
			<div className="flex-1 flex flex-col justify-safe-center overflow-auto pt-32">
				{back ? (
					<motion.div
						className="max-w-5xl mt-16 mx-auto w-full"
						initial={{ opacity: 0, y: -20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.3, delay: 0.5 }}
					>
						<Button
							className="mb-4 text-muted-foreground px-0.5 py-1 h-auto -mx-0.5"
							startIcon={<Icon icon={faChevronLeft} />}
							variant="link"
							size="xs"
							asChild
						>
							{back}
						</Button>
					</motion.div>
				) : null}
				<div className="max-w-5xl flex mx-auto flex-1 items-center justify-center">
					<LayoutGroup>
						<div>
							<motion.div
								layout
								className="mt-2 justify-between items-center px-10 py-4"
							>
								<H1 className="text-center">
									Get started with Rivet
								</H1>
								<p className="text-center text-muted-foreground mt-2 max-w-md w-full mx-auto">
									Choose a template to quickly set up your
									project, or connect an existing one.
								</p>
							</motion.div>
							<div>
								<Templates {...props} />
							</div>
						</div>
					</LayoutGroup>
				</div>
				<OnboardingFooter />
			</div>
		</div>
	);
}

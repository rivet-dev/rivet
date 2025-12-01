import { useUser } from "@clerk/clerk-react";
import { createFileRoute } from "@tanstack/react-router";
import { LayoutGroup, motion } from "framer-motion";
import { Templates } from "@/app/getting-started";
import { Logo } from "@/app/layout";
import { Button, H1 } from "@/components";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/new/",
)({
	component: RouteComponent,
});

function RouteComponent() {
	const { user } = useUser();

	return (
		<>
			<div className="rounded-lg flex items-center px-6 justify-between bg-card/10 backdrop-blur-lg fixed inset-x-0 top-0 z-10 h-16 border-b">
				<Logo />
				<Button
					variant="ghost"
					className="text-sm text-muted-foreground font-normal"
				>
					Logged in as{" "}
					<span className="text-foreground">
						{user?.primaryEmailAddress?.emailAddress}
					</span>
				</Button>
			</div>
			<div className="h-screen flex flex-col justify-safe-center">
				<div className="flex-1 flex flex-col justify-safe-center overflow-auto">
					<div className="max-w-5xl mx-auto">
						<LayoutGroup>
							<motion.div
								layout
								className="mt-2 justify-between items-center px-10 py-4 pt-20"
							>
								<H1 className="text-center">Get Started</H1>
								<p className="text-center text-muted-foreground mt-2 max-w-md w-full mx-auto">
									Choose a template to quickly set up your
									project, or connect an existing one.
								</p>
							</motion.div>
							<div>
								<Templates
									getTemplateLink={(template) => ({
										to: "/orgs/$organization/new/$template",
										params: (old) => ({ ...old, template }),
										viewTransition: true,
									})}
								/>
							</div>
						</LayoutGroup>
					</div>
				</div>
			</div>
		</>
	);
}

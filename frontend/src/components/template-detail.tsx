import { faArrowUpRight, faChevronLeft, Icon } from "@rivet-gg/icons";
import type { Template } from "@rivetkit/example-registry";
import { Link } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { match } from "ts-pattern";
import CreateProjectFrameContent from "@/app/dialogs/create-project-frame";
import { ExamplePreview } from "@/app/templates";
import { FrameConfigProvider } from "./hooks/isomorphic-frame";
import { Button } from "./ui/button";
import { Card, CardHeader } from "./ui/card";
import { H1 } from "./ui/typography";

export function TemplateDetail({
	template,
	organization,
}: {
	template: Template;
	organization: string;
}) {
	return (
		<div className="h-screen flex flex-col justify-safe-center">
			<div className="flex-1 flex flex-col justify-safe-center overflow-auto">
				<div className="max-w-2xl mx-auto w-full"></div>
				<div className="max-w-2xl flex mx-auto flex-1 rounded-md px-6 items-center justify-center">
					<div>
						<motion.div
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
								<Link to="..">Back to Templates</Link>
							</Button>
						</motion.div>
						<Card className="mx-auto overflow-hidden">
							<CardHeader className="p-0 mb-4">
								<div className="max-w-4xl relative">
									<ExamplePreview
										className="relative bottom-0 aspect-video mx-auto rounded-t-none rounded-b-none border-t-0 border-l-0 border-r-0"
										slug={template.name}
										title={template.displayName}
									/>
									<div className="absolute bottom-4 inset-x-0 justify-between items-center px-10 py-4 pt-20">
										<H1 className="text-center">
											Get Started with{" "}
											{template.displayName}
										</H1>
										<p className="text-center text-muted-foreground mt-2 max-w-md w-full mx-auto">
											{template.description}
										</p>
									</div>
								</div>
							</CardHeader>
							<CreateProject
								template={template}
								organization={organization}
							/>
						</Card>
					</div>
				</div>
				<Links />
			</div>
		</div>
	);
}

export function Links() {
	return (
		<div className="flex gap-4 justify-center py-8 bg-gradient-to-t from-background to-transparent">
			<Button
				variant="link"
				size="xs"
				className="text-muted-foreground"
				endIcon={<Icon icon={faArrowUpRight} className="ms-1" />}
			>
				<a
					href="http://rivet.dev/docs"
					target="_blank"
					rel="noopener noreferrer"
				>
					Documentation
				</a>
			</Button>
			{match(__APP_TYPE__)
				.with("cloud", () => (
					<Button
						className="text-muted-foreground"
						variant="link"
						size="xs"
						onClick={() => {
							Plain.open();
						}}
						endIcon={
							<Icon icon={faArrowUpRight} className="ms-1" />
						}
					>
						Support
					</Button>
				))
				.otherwise(() => (
					<Button
						className="text-muted-foreground"
						variant="link"
						size="xs"
						asChild
						endIcon={
							<Icon icon={faArrowUpRight} className="ms-1" />
						}
					>
						<Link
							to="."
							search={(old) => ({
								...old,
								modal: "feedback",
							})}
						>
							Feedback
						</Link>
					</Button>
				))}
			<Button
				variant="link"
				size="xs"
				className="text-muted-foreground"
				endIcon={<Icon icon={faArrowUpRight} className="ms-1" />}
			>
				<a
					href="http://rivet.gg/discord"
					target="_blank"
					rel="noopener noreferrer"
				>
					Discord
				</a>
			</Button>
			<Button
				variant="link"
				size="xs"
				className="text-muted-foreground"
				endIcon={<Icon icon={faArrowUpRight} className="ms-1" />}
			>
				<a
					href="http://github.com/rivet-gg/rivet"
					target="_blank"
					rel="noopener noreferrer"
				>
					GitHub
				</a>
			</Button>
		</div>
	);
}

function CreateProject({
	template,
	organization,
}: {
	template: Template;
	organization: string;
}) {
	return (
		<FrameConfigProvider
			value={{
				showHeader: false,
				contentClassName: "px-32",
				footerClassName: "px-32 justify-end",
			}}
		>
			<CreateProjectFrameContent
				organization={organization}
				template={template.name}
			/>
		</FrameConfigProvider>
	);
}

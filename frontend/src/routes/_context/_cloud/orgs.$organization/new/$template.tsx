import { useUser } from "@clerk/clerk-react";
import {} from "@fortawesome/free-solid-svg-icons";
import { faArrowUpRight, faChevronLeft, Icon } from "@rivet-gg/icons";
import { templates } from "@rivetkit/example-registry";
import { createFileRoute, Link, redirect } from "@tanstack/react-router";
import { LayoutGroup, motion } from "framer-motion";
import { match } from "ts-pattern";
import * as StartNewExampleForm from "@/app/forms/start-with-new-example-form";
import { ExamplePreview, Templates } from "@/app/getting-started";
import { Logo } from "@/app/layout";
import {
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	H1,
} from "@/components";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/new/$template",
)({
	component: RouteComponent,
	loader: async (context) => {
		const { template } = context.params;

		const templateData = templates.find((t) => t.name === template);
		if (!templateData) {
			throw redirect({
				to: "/orgs/$organization/new",
				params: context.params,
			});
		}

		return {
			template: templateData,
		};
	},
	loaderDeps(opts) {
		return [];
	},
});

function RouteComponent() {
	const { template } = Route.useLoaderData();
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
						<Card className="mx-auto my-8 overflow-hidden">
							<CardHeader className="p-0">
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
							<CardContent className="mx-auto mt-8 max-w-2xl">
								<div className="grid grid-cols-[1fr_2fr] gap-4">
									<div className="flex flex-col gap-1 items-end border-r pr-2">
										<Button
											asChild
											variant="ghost"
											className="text-muted-foreground"
											startIcon={
												<Icon icon={faChevronLeft} />
											}
										>
											<Link
												to={"/orgs/$organization/new"}
												params={Route.useParams()}
											>
												Back to templates
											</Link>
										</Button>
										<Button
											variant="ghost"
											className="text-muted-foreground"
											endIcon={
												<Icon
													icon={faArrowUpRight}
													className="ms-1"
												/>
											}
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
													variant="ghost"
													onClick={() => {
														Plain.open();
													}}
													endIcon={
														<Icon
															icon={
																faArrowUpRight
															}
															className="ms-1"
														/>
													}
												>
													Support
												</Button>
											))
											.otherwise(() => (
												<Button
													className="text-muted-foreground"
													variant="ghost"
													asChild
													endIcon={
														<Icon
															icon={
																faArrowUpRight
															}
															className="ms-1"
														/>
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
											variant="ghost"
											className="text-muted-foreground"
											endIcon={
												<Icon
													icon={faArrowUpRight}
													className="ms-1"
												/>
											}
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
											variant="ghost"
											className="text-muted-foreground"
											endIcon={
												<Icon
													icon={faArrowUpRight}
													className="ms-1"
												/>
											}
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
									<div className="flex flex-col gap-2 py-2">
										<StartNewExampleForm.Form
											defaultValues={{
												projectName:
													template.displayName,
											}}
										>
											<StartNewExampleForm.Organization />
											<StartNewExampleForm.ProjectName
												placeholder={
													template.displayName
												}
											/>
											<div className="flex mt-4 justify-end">
												<StartNewExampleForm.Submit>
													Get Started
												</StartNewExampleForm.Submit>
											</div>
										</StartNewExampleForm.Form>
									</div>
								</div>
							</CardContent>
						</Card>
					</div>
				</div>
			</div>
		</>
	);
}

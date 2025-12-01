import { Link } from "@tanstack/react-router";
import { cn } from "../components/lib/utils";
import { Button } from "../components/ui/button";
import { H2 } from "../components/ui/typography";

export function GettingStarted() {
	return (
		<div className="h-full border my-2 mr-2 px-4 py-4 rounded-lg flex flex-col items-center justify-safe-center overflow-auto @container">
			<div className="max-w-2xl w-full mb-6">
				<div className="border rounded-lg w-full bg-card">
					<div className="mt-2 flex justify-between items-center px-6 py-4 sticky top-0">
						<H2>Start With Template</H2>
					</div>

					<hr />
					<div className="p-4 px-6">
						<div className="grid grid-cols-2 @4xl:grid-cols-3 gap-2 my-4">
							<TemplateCard
								slug="chat-room"
								title="Chat Room Template"
								description="Example project demonstrating real-time messaging and actor state management."
								className="col-span-full @4xl:col-span-1 @4xl:col-start-2"
							/>
						</div>
					</div>
				</div>
				<div className="max-w-3xl w-full mt-4 flex justify-stretch gap-4">
					<Button
						variant="secondary"
						className="w-full bg-card py-4 h-auto"
					>
						<Link
							to="."
							search={{ modal: "connect-existing-project" }}
						>
							Connect Existing Project
						</Link>
					</Button>
					<Button
						variant="secondary"
						className="w-full bg-card py-4 h-auto"
						asChild
					>
						<a
							href="https://www.rivet.dev/docs"
							rel="noopener noreferrer"
							target="_blank"
						>
							Create New Project
						</a>
					</Button>
				</div>
			</div>
		</div>
	);
}

function TemplateCard({
	title,
	slug,
	className,
	description,
}: {
	title: string;
	slug: string;
	description?: string;
	className?: string;
}) {
	return (
		<Button
			size="lg"
			variant="outline"
			className={cn(
				"h-auto pb-2 px-0 flex-col items-start text-wrap",
				className,
			)}
			asChild
		>
			<Link
				to="."
				search={{ modal: "start-with-template", template: slug }}
			>
				<div className="border-b min-h-40 w-full"></div>
				<div className="px-4 py-2 gap-1.5 flex flex-col">
					<p className="text-base">{title}</p>
					{description && (
						<p className="text-muted-foreground text-xs line-clamp-2">
							{description}
						</p>
					)}
				</div>
			</Link>
		</Button>
	);
}

import { faNextjs, faNodeJs, faReact, Icon } from "@rivet-gg/icons";
import { useSearch } from "@tanstack/react-router";
import type { ComponentProps, Ref } from "react";
import {
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	DocsSheet,
} from "@/components";
import { ConnectionForm } from "@/components/connection-form";
import { docsLinks } from "@/content/data";

export function Connect({
	onSubmit,
	formRef,
}: {
	formRef?: Ref<HTMLFormElement>;
	onSubmit: ComponentProps<typeof ConnectionForm>["onSubmit"];
}) {
	const search = useSearch({ from: "/_context" });
	return (
		<>
			<Card className="sm:w-96 w-full mb-6">
				<CardHeader>
					<CardTitle>Getting Started</CardTitle>
				</CardHeader>
				<CardContent>
					<p>Get started with one of our quick start guides:</p>
					<div className="flex-1 flex flex-col gap-2 mt-4">
						<div className="flex flex-row flex-wrap justify-stretch items-center gap-2">
							<DocsSheet
								path={docsLinks.gettingStarted.js}
								title="Node.js & Bun Quickstart"
							>
								<Button
									className="flex-1"
									variant="outline"
									startIcon={<Icon icon={faNodeJs} />}
								>
									Node.js & Bun
								</Button>
							</DocsSheet>
							<DocsSheet
								path={docsLinks.gettingStarted.react}
								title="React Quickstart"
							>
								<Button
									className="flex-1"
									variant="outline"
									startIcon={<Icon icon={faReact} />}
								>
									React
								</Button>
							</DocsSheet>

							<DocsSheet
								path={docsLinks.gettingStarted.nextjs}
								title="Next.js Quickstart"
							>
								<Button
									className="flex-1"
									variant="outline"
									startIcon={<Icon icon={faNextjs} />}
								>
									Next.js
								</Button>
							</DocsSheet>
						</div>
					</div>
				</CardContent>
			</Card>

			<Card className="sm:w-96">
				<CardHeader>
					<CardTitle>Connect to Project</CardTitle>
				</CardHeader>
				<CardContent>
					<p className="mb-4">
						Connect to your Rivet Project by entering your RivetKit
						URL.
					</p>

					<ConnectionForm
						ref={formRef}
						defaultValues={{
							url: search.u || "http://localhost:6420",
						}}
						onSubmit={onSubmit}
					/>
				</CardContent>
			</Card>
		</>
	);
}

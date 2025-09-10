import { faNodeJs, faReact, Icon } from "@rivet-gg/icons";
import { createFileRoute, notFound, Outlet } from "@tanstack/react-router";
import {
	type ComponentProps,
	type Ref,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { match } from "ts-pattern";
import { InspectorCredentialsProvider } from "@/app/credentials-context";
import { createClient } from "@/app/data-providers/inspector-data-provider";
import {
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	DocsSheet,
	H1,
} from "@/components";
import { ConnectionForm } from "@/components/connection-form";
import { ModalRenderer } from "@/components/modal-renderer";
import { docsLinks } from "@/content/data";

export const Route = createFileRoute("/_context/_inspector")({
	component: RouteComponent,
	context: ({ context, params }) => {
		return match(context)
			.with({ __type: "inspector" }, (ctx) => ({
				dataProvider: {
					...ctx.dataProvider,
				},
			}))
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			});
	},
	beforeLoad: () => {
		return match(__APP_TYPE__)
			.with("inspector", async () => {})
			.otherwise(() => {
				throw notFound();
			});
	},
});

function RouteComponent() {
	const search = Route.useSearch();
	const [credentials, setCredentials] = useState<null | {
		url: string;
		token: string;
	}>(null);

	const formRef = useRef<HTMLFormElement>(null);

	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to run this only once on mount
	useEffect(() => {
		if (search.t) {
			formRef.current?.requestSubmit();
		}
	}, []);

	const ctxValue = useMemo(() => {
		return { credentials, setCredentials };
	}, [credentials]);

	if (credentials) {
		return (
			<InspectorCredentialsProvider value={ctxValue}>
				<ModalRenderer />
				<Outlet />
			</InspectorCredentialsProvider>
		);
	}

	return (
		<Connect
			formRef={formRef}
			onSubmit={async (values, form) => {
				try {
					const client = createClient({
						url: values.username,
						token: values.token,
					});
					const resp = await client.ping.$get();
					if (!resp.ok) {
						throw resp;
					}
					setCredentials({
						url: values.username,
						token: values.token,
					});
				} catch {
					form.setError("token", {
						message:
							"Failed to connect. Please check your URL and token.",
					});
				}
			}}
		/>
	);
}

function Connect({
	onSubmit,
	formRef,
}: {
	formRef?: Ref<HTMLFormElement>;
	onSubmit: ComponentProps<typeof ConnectionForm>["onSubmit"];
}) {
	const search = Route.useSearch();
	return (
		<div className="w-full h-screen flex flex-col items-center justify-center">
			<H1 className="mb-8">Rivet Inspector</H1>
			<Card className="max-w-md w-full mb-6">
				<CardHeader>
					<CardTitle>Getting Started</CardTitle>
				</CardHeader>
				<CardContent>
					<p>Get started with one of our quick start guides:</p>
					<div className="flex-1 flex flex-col gap-2 mt-4">
						<div className="flex flex-row justify-stretch items-center gap-2">
							<DocsSheet
								path={docsLinks.gettingStarted.node}
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
						</div>
					</div>
				</CardContent>
			</Card>

			<Card className="max-w-md w-full mb-6">
				<CardHeader>
					<CardTitle>Connect to Project</CardTitle>
				</CardHeader>
				<CardContent>
					<p className="mb-4">
						Connect to your RivetKit project by entering the URL and
						access token.
					</p>

					<ConnectionForm
						ref={formRef}
						defaultValues={{
							username: search.u || "http://localhost:8080",
							token: search.t || "",
						}}
						onSubmit={onSubmit}
					/>
				</CardContent>
			</Card>
		</div>
	);
}

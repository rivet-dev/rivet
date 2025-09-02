import { faNodeJs, faReact, Icon } from "@rivet-gg/icons";
import { createFileRoute, Outlet, useMatch } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { usePostHog } from "posthog-js/react";
import {
	type ComponentProps,
	type ReactNode,
	type Ref,
	Suspense,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import z from "zod";
import {
	type InspectorCredentials,
	InspectorCredentialsProvider,
} from "@/app/credentials-context";
import * as Layout from "@/app/layout";
import { useDialog } from "@/app/use-dialog";
import {
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	DocsSheet,
	FEEDBACK_FORM_ID,
	H1,
	type ImperativePanelHandle,
} from "@/components";
import { ActorProvider, ManagerProvider } from "@/components/actors";
import { RootLayoutContextProvider } from "@/components/actors/root-layout-context";
import { ConnectionForm } from "@/components/connection-form";
import { docsLinks } from "@/content/data";
import { createEngineActorContext } from "@/queries/actor-engine";
import { createInspectorActorContext } from "@/queries/actor-inspector";
import {
	createEngineManagerContext,
	type NamespaceNameId,
} from "@/queries/manager-engine";
import {
	createClient,
	createInspectorManagerContext,
} from "@/queries/manager-inspector";

const searchSchema = z
	.object({
		modal: z
			.enum(["go-to-actor", "feedback", "create-ns"])
			.or(z.string())
			.optional(),
		utm_source: z.string().optional(),
		actorId: z.string().optional(),
		tab: z.string().optional(),
		n: z.array(z.string()).optional(),
		u: z.string().optional(),
		t: z.string().optional(),
	})
	.and(z.record(z.string(), z.any()));

export const Route = createFileRoute("/_layout")({
	validateSearch: zodValidator(searchSchema),
	component: RouteComponent,
});

function RouteComponent() {
	const match = useMatch({
		from: "/_layout/ns/$namespace",
		shouldThrow: false,
	});

	const sidebarRef = useRef<ImperativePanelHandle>(null);
	const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);

	const content = (
		<>
			<Suspense>
				<Modals />
			</Suspense>
			<Layout.Root>
				<Layout.VisibleInFull>
					<Layout.Sidebar
						ref={sidebarRef}
						onCollapse={() => {
							setIsSidebarCollapsed(true);
						}}
						onExpand={() => setIsSidebarCollapsed(false)}
					/>
					<Layout.Main>
						<RootLayoutContextProvider
							sidebarRef={sidebarRef}
							isSidebarCollapsed={isSidebarCollapsed}
						>
							<Outlet />
						</RootLayoutContextProvider>
					</Layout.Main>
				</Layout.VisibleInFull>
				<Layout.Footer />
			</Layout.Root>
		</>
	);

	if (match?.params.namespace) {
		return (
			<ContextContent
				namespace={match.params.namespace}
				content={content}
			/>
		);
	}

	if (__APP_TYPE__ === "inspector") {
		return <InspectorContent content={content} />;
	}

	return content;
}

function ContextContent({
	namespace,
	content,
}: {
	namespace: string;
	content: React.ReactNode;
}) {
	const managerContext = useMemo(() => {
		return createEngineManagerContext({ namespace });
	}, [namespace]);

	return <ManagerProvider value={managerContext}>{content}</ManagerProvider>;
}

function Modals() {
	const search = Route.useSearch();
	const navigate = Route.useNavigate();

	const posthog = usePostHog();

	const FeedbackDialog = useDialog.Feedback.Dialog;
	const CreateNamespaceDialog = useDialog.CreateNamespace.Dialog;

	const GoToActorDialog = useDialog.GoToActor.Dialog;
	const CreateActorDialog = useDialog.CreateActor.Dialog;

	const match = useMatch({
		from: "/_layout/ns/$namespace",
		shouldThrow: false,
	});

	const { modal, utm_source } = search;

	const handleOnOpenChange = (value: boolean) => {
		if (!value) {
			navigate({
				to: ".",
				search: (old) => ({ ...old, modal: undefined }),
			});
		} else {
			posthog.capture("survey shown", { $survey_id: FEEDBACK_FORM_ID });
		}
	};

	return (
		<>
			<FeedbackDialog
				source={utm_source}
				dialogProps={{
					open: modal === "feedback",
					onOpenChange: handleOnOpenChange,
				}}
			/>
			{__APP_TYPE__ === "engine" ? (
				<CreateNamespaceDialog
					dialogProps={{
						open: modal === "create-ns",
						onOpenChange: (value) => {
							if (!value) {
								navigate({
									to: ".",
									search: (old) => ({
										...old,
										modal: undefined,
									}),
								});
							}
						},
					}}
				/>
			) : null}
			<GoToActorDialog
				onSubmit={(actorId) => {
					navigate({
						to: ".",
						search: (old) => ({
							...old,
							actorId,
							modal: undefined,
						}),
					});
				}}
				dialogProps={{
					open: modal === "go-to-actor",
					onOpenChange: (value) => {
						if (!value) {
							navigate({
								search: (old) => ({
									...old,
									modal: undefined,
								}),
							});
						}
					},
				}}
			/>
			<CreateActorDialog
				namespace={match?.params.namespace as NamespaceNameId}
				dialogProps={{
					open: modal === "create-actor",
					onOpenChange: (value) => {
						if (!value) {
							navigate({
								search: (old) => ({
									...old,
									modal: undefined,
								}),
							});
						}
					},
				}}
			/>
		</>
	);
}

function InspectorContent({ content }: { content: ReactNode }) {
	const search = Route.useSearch();
	const [credentials, setCredentials] = useState<null | {
		url: string;
		token: string;
	}>(null);

	const formRef = useRef<HTMLFormElement>(null);

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
				<InspectorContextContent
					content={content}
					credentials={credentials}
				/>
			</InspectorCredentialsProvider>
		);
	}

	return (
		<Connect
			formRef={formRef}
			onSubmit={async (values, form) => {
				try {
					const client = createClient(values.username, values.token);
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

function InspectorContextContent({
	content,
	credentials,
}: {
	credentials: InspectorCredentials;
	content: ReactNode;
}) {
	const managerContext = useMemo(() => {
		return createInspectorManagerContext(credentials);
	}, [credentials]);

	return <ManagerProvider value={managerContext}>{content}</ManagerProvider>;
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

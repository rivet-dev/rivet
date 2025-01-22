import { ErrorComponent } from "@/components/error-component";
import { NotFoundComponent } from "@/components/not-found-component";
import type { AuthContext } from "@/domains/auth/contexts/auth";
import { useDialog } from "@/hooks/use-dialog";
import * as Layout from "@/layouts/root";
import { FEEDBACK_FORM_ID } from "@/lib/data/constants";
import { FullscreenLoading, Page } from "@rivet-gg/components";
import { PageLayout } from "@rivet-gg/components/layout";
import type { QueryClient } from "@tanstack/react-query";
import {
	type ErrorComponentProps,
	Outlet,
	createRootRouteWithContext,
} from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/router-devtools";
import { zodValidator } from "@tanstack/zod-adapter";
import { usePostHog } from "posthog-js/react";
import { useKonami } from "react-konami-code";
import { z } from "zod";

function Modals() {
	const search = Route.useSearch();
	const navigate = Route.useNavigate();

	const posthog = usePostHog();

	const FeedbackDialog = useDialog.Feedback.Dialog;
	const SecretDialog = useDialog.Secret.Dialog;

	useKonami(() => navigate({ search: { modal: "secret" } }));

	if (!search || !("modal" in search)) {
		return;
	}

	const { modal, utm_source } = search;

	const handleonOpenChange = (value: boolean) => {
		if (!value) {
			navigate({ search: { modal: undefined } });
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
					onOpenChange: handleonOpenChange,
				}}
			/>
			<SecretDialog
				dialogProps={{
					open: modal === "secret",
					onOpenChange: handleonOpenChange,
				}}
			/>
		</>
	);
}

function RootNotFoundComponent() {
	return (
		<PageLayout.Root>
			<Page title="Not found">
				<NotFoundComponent />
			</Page>
		</PageLayout.Root>
	);
}

function RootErrorComponent(props: ErrorComponentProps) {
	return (
		<PageLayout.Root>
			<Page title="Error!">
				<ErrorComponent {...props} />
			</Page>
		</PageLayout.Root>
	);
}

function Root() {
	return (
		<Layout.Root>
			<Layout.VisibleInFull>
				<Layout.Header />
				<Layout.Main>
					<Modals />
					<Outlet />
				</Layout.Main>
			</Layout.VisibleInFull>
			<Layout.Footer />
		</Layout.Root>
	);
}

function RootRoute() {
	return (
		<>
			<Root />

			{import.meta.env.DEV ? <TanStackRouterDevtools /> : null}
		</>
	);
}

export interface RouterContext {
	auth: AuthContext;
	queryClient: QueryClient;
	subNav?: { title: string; url: string; exact?: boolean }[];
}

const searchSchema = z.object({
	modal: z
		.enum(["secret", "feedback"])
		.optional()
		.catch(({ input }) => input),
	utm_source: z.string().optional(),
});

export const Route = createRootRouteWithContext<RouterContext>()({
	validateSearch: zodValidator(searchSchema),
	component: RootRoute,
	errorComponent: RootErrorComponent,
	notFoundComponent: RootNotFoundComponent,
	pendingComponent: FullscreenLoading,
	wrapInSuspense: true,
});

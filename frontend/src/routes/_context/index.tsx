import { createFileRoute, isRedirect, redirect } from "@tanstack/react-router";
import { match } from "ts-pattern";
import CreateNamespacesFrameContent from "@/app/dialogs/create-namespace-frame";
import { InspectorContextProvider } from "@/app/inspector-context";
import { InspectorRoot } from "@/app/inspector-root";
import { Logo } from "@/app/logo";
import { Card } from "@/components";
import { redirectToOrganization } from "@/lib/auth";

export const Route = createFileRoute("/_context/")({
	component: () =>
		match(__APP_TYPE__)
			.with("cloud", () => <CloudRoute />)
			.with("engine", () => <EngineRoute />)
			.with("inspector", () => <InspectorRoute />)
			.exhaustive(),
	beforeLoad: async ({ context, search }) => {
		return await match(context)
			.with({ __type: "cloud" }, async () => {
				if (!(await redirectToOrganization(context.clerk, search))) {
					throw redirect({ to: "/login", search: true });
				}
			})
			.with({ __type: "engine" }, async (ctx) => {
				try {
					const result = await ctx.queryClient.fetchInfiniteQuery(
						ctx.dataProvider.namespacesQueryOptions(),
					);

					const firstNamespace = result.pages[0]?.namespaces[0];
					if (!firstNamespace) {
						throw redirect({
							to: "/ns/$namespace",
							params: { namespace: "default" },
							search: true,
						});
					}
					throw redirect({
						to: "/ns/$namespace",
						params: { namespace: firstNamespace.name },
						search: true,
					});
				} catch (e) {
					if (isRedirect(e)) {
						throw e;
					}

					// Ignore errors here, they will be handled in the UI
					return;
				}
			})
			.with({ __type: "inspector" }, async () => {
				return {};
			})
			.exhaustive();
	},
});

function CloudRoute() {
	return null;
}

function EngineRoute() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6 w-full">
				<Logo className="h-10 mb-4" />
				<Card className="w-full sm:w-96">
					<CreateNamespacesFrameContent />
				</Card>
			</div>
		</div>
	);
}

function InspectorRoute() {
	return (
		<InspectorContextProvider>
			<InspectorRoot />
		</InspectorContextProvider>
	);
}

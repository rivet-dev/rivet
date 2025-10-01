import {
	createFileRoute,
	notFound,
	Outlet,
	redirect,
} from "@tanstack/react-router";
import { match } from "ts-pattern";

export const Route = createFileRoute("/_context/")({
	component: () => <Outlet />,
	beforeLoad: (ctx) => {
		return match(ctx.context)
			.with({ __type: "cloud" }, () => {
				const { organization } = ctx.context.clerk ?? {};
				if (!organization) {
					throw notFound();
				}
				throw redirect({
					to: "/orgs/$organization",
					params: { organization: organization?.id },
					reloadDocument: true,
				});
			})
			.with({ __type: "engine" }, async (ctx) => {
				const result = await ctx.queryClient.fetchInfiniteQuery(
					ctx.dataProvider.namespacesQueryOptions(),
				);

				const firstNamespace = result.pages[0]?.namespaces[0];
				if (!firstNamespace) {
					throw notFound();
				}

				throw redirect({
					to: "/ns/$namespace",
					params: { namespace: firstNamespace.name },
					reloadDocument: true,
				});
			})
			.with({ __type: "inspector" }, async () => {})
			.exhaustive();
	},
});

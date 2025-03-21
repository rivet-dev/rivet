import { LoginView } from "@/domains/auth/views/login-view/login-view";
import * as Layout from "@/layouts/page-centered";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { fallback, zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";

const searchSchema = z.object({
	redirect: fallback(z.string(), "/").catch("/"),
});

export const Route = createFileRoute("/login")({
	validateSearch: zodValidator(searchSchema),
	wrapInSuspense: true,
	pendingComponent: Layout.Root.Skeleton,
	component: () => {
		const search = Route.useSearch();
		const navigate = useNavigate();
		return (
			<Layout.Root>
				<LoginView
					onSuccess={() => {
						navigate({
							to: search.redirect || "/",
						});
					}}
				/>
			</Layout.Root>
		);
	},
});

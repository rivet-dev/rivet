import { LoginView } from "@/domains/auth/views/login-view/login-view";
import * as Layout from "@/layouts/page-centered";
import { createFileRoute, redirect, useNavigate } from "@tanstack/react-router";
import { fallback, zodValidator } from "@tanstack/zod-adapter";
import { startTransition } from "react";
import { z } from "zod";

const searchSchema = z.object({
	redirect: fallback(z.string(), "/").catch("/"),
});

export const Route = createFileRoute("/login")({
	validateSearch: zodValidator(searchSchema),
	beforeLoad: ({ context: { auth }, search }) => {
		if (auth.profile?.identity.isRegistered) {
			throw redirect({
				to: search.redirect || "/",
			});
		}
	},
	wrapInSuspense: true,
	pendingComponent: Layout.Root.Skeleton,
	component: () => {
		const search = Route.useSearch();
		const navigate = useNavigate();
		return (
			<Layout.Root>
				<LoginView
					onSuccess={() => {
						startTransition(() => {
							navigate({
								to: search.redirect || "/",
							});
						});
					}}
				/>
			</Layout.Root>
		);
	},
});

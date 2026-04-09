import { createFileRoute, redirect } from "@tanstack/react-router";
import { toast } from "sonner";
import { z } from "zod";
import { Login } from "@/app/login";
import { Logo } from "@/app/logo";
import { authClient, redirectToOrganization } from "@/lib/auth";

export const Route = createFileRoute("/login")({
	component: RouteComponent,
	validateSearch: z.object({ emailVerified: z.coerce.number().optional() }),
	beforeLoad: async ({ search }) => {
		if (search.emailVerified) {
			toast.success("Email verified successfully. You can now sign in.", {
				position: "top-center",
			});
			throw redirect({ to: ".", search: { emailVerified: undefined } });
		}

		const session = await authClient.getSession();
		if (session.data) {
			await redirectToOrganization({
				from: "from" in search ? (search.from as string) : undefined,
			});
		}
	},
});

function RouteComponent() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6 w-full">
				<Logo className="h-10 mb-4" />
				<Login />
				<p className="max-w-md text-center text-xs text-muted-foreground">
					Looking for Rivet Enterprise Cloud? Visit{" "}
					<a
						href="https://hub.rivet.gg"
						className="underline"
						target="_blank"
						rel="noreferrer"
					>
						hub.rivet.gg
					</a>
				</p>
			</div>
		</div>
	);
}

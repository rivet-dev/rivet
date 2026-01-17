import { createFileRoute, redirect } from "@tanstack/react-router";
import { Logo } from "@/app/logo";
import { SignUp } from "@/app/sign-up";
import { waitForClerk } from "@/lib/waitForClerk";

export const Route = createFileRoute("/join")({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		await waitForClerk(context.clerk);
		if (context.clerk.user) {
			throw redirect({ to: "/", search: true });
		}
	},
});

function RouteComponent() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6">
				<Logo className="h-10 mb-4" />
				<SignUp />
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

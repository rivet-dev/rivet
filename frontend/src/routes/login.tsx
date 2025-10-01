import { createFileRoute, redirect } from "@tanstack/react-router";
import { Login } from "@/app/login";
import { Logo } from "@/app/logo";
import { waitForClerk } from "@/lib/waitForClerk";

export const Route = createFileRoute("/login")({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		await waitForClerk(context.clerk);
		if (context.clerk.user) {
			throw redirect({ to: "/" });
		}
	},
});

function RouteComponent() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6">
				<Logo className="h-10 mb-4" />
				<Login />
			</div>
		</div>
	);
}

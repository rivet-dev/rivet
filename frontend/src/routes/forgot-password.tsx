import { createFileRoute } from "@tanstack/react-router";
import { ForgotPassword } from "@/app/forgot-password";
import { Logo } from "@/app/logo";

export const Route = createFileRoute("/forgot-password")({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6 w-full">
				<Logo className="h-10 mb-4" />
				<ForgotPassword />
			</div>
		</div>
	);
}

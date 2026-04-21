import { createFileRoute } from "@tanstack/react-router";
import { Logo } from "@/app/logo";
import { ResetPassword } from "@/app/reset-password";

export const Route = createFileRoute("/reset-password")({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6 w-full">
				<Logo className="h-10 mb-4" />
				<ResetPassword />
			</div>
		</div>
	);
}

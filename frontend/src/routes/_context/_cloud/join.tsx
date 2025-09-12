import { createFileRoute } from "@tanstack/react-router";
import { Logo } from "@/app/logo";
import { SignUp } from "@/app/sign-up";

export const Route = createFileRoute("/_context/_cloud/join")({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6">
				<Logo className="h-10 mb-4" />
				<SignUp />
			</div>
		</div>
	);
}

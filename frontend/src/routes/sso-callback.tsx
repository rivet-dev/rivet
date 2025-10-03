import { AuthenticateWithRedirectCallback } from "@clerk/clerk-react";
import { createFileRoute } from "@tanstack/react-router";
import { FullscreenLoading } from "@/components";

export const Route = createFileRoute("/sso-callback")({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<>
			<FullscreenLoading>
				{/** biome-ignore lint/correctness/useUniqueElementIds: required by clerk */}
				<div id="clerk-captcha" />
			</FullscreenLoading>
			<AuthenticateWithRedirectCallback />
		</>
	);
}

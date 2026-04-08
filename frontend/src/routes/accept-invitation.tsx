import { createFileRoute } from "@tanstack/react-router";
import { AcceptInvitation } from "@/app/accept-invitation";
import { features } from "@/lib/features";

export const Route = createFileRoute("/accept-invitation")({
	component: features.auth ? AcceptInvitation : () => null,
});

import { createFileRoute } from "@tanstack/react-router";
import { VerifyEmail } from "@/app/verify-email";

export const Route = createFileRoute("/verify-email")({
	component: VerifyEmail,
});

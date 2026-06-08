import { createFileRoute } from "@tanstack/react-router";
import { NewOrgPage } from "@/app/new-org-page";

export const Route = createFileRoute("/_context/new-org/")({
	component: NewOrgPage,
});

import { createFileRoute } from "@tanstack/react-router";
import { LayoutMockup } from "@/components/actors/mockup/layout-mockup";

export const Route = createFileRoute(
	"/_context/_engine/ns/$namespace/mockup",
)({
	component: LayoutMockup,
});

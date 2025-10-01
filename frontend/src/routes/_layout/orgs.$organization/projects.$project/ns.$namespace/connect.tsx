import { createFileRoute, notFound } from "@tanstack/react-router";
import { match } from "ts-pattern";

export const Route = createFileRoute(
	"/_layout/orgs/$organization/projects/$project/ns/$namespace/connect",
)({
	component: match(__APP_TYPE__)
		.with("cloud", () => RouteComponent)
		.otherwise(() => () => {
			throw notFound();
		}),
});

function RouteComponent() {
	return (
		<div>
			Hello
			"/_layout/orgs/$organization/projects/$project/ns/$namespace/connect"!
		</div>
	);
}

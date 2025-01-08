import { ProjectSelect } from "@/domains/project/components/project-select";
import { projectQueryOptions } from "@/domains/project/queries";
import * as Layout from "@/layouts/page-centered";
import { guardEnterprise } from "@/lib/guards";
import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@rivet-gg/components";
import { useQuery } from "@tanstack/react-query";
import { Link, createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

function BillingRoute() {
	const [projectId, setProjectId] = useState<string | null>(null);

	const { data, isLoading } = useQuery({
		// biome-ignore lint/style/noNonNullAssertion: it's safe to assume that projectId exists
		...projectQueryOptions(projectId!),
		enabled: !!projectId,
	});

	return (
		<Layout.Root>
			<Card>
				<CardHeader>
					<CardTitle>Billing</CardTitle>
					<CardDescription>
						Choose a project for which you would like to manage
						billing.
					</CardDescription>
				</CardHeader>
				<CardContent>
					<ProjectSelect onValueChange={setProjectId} />
				</CardContent>
				<CardFooter>
					<Button asChild disabled={!projectId} isLoading={isLoading}>
						<Link
							to="/projects/$projectNameId/billing"
							/* biome-ignore lint/style/noNonNullAssertion: it's safe to assume that projectid exists */
							params={{ projectNameId: data?.nameId! }}
						>
							Manage billing
						</Link>
					</Button>
				</CardFooter>
			</Card>
		</Layout.Root>
	);
}

export const Route = createFileRoute("/_authenticated/billing")({
	beforeLoad: async ({ context: { queryClient } }) => {
		await guardEnterprise({ queryClient });
	},
	component: BillingRoute,
});

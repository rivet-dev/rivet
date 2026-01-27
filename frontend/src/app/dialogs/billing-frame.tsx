import { useSuspenseQueries } from "@tanstack/react-query";
import { Frame, Link } from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";
import { BillingPlans } from "../billing/billing-plans";
import { BillingStatus } from "../billing/billing-status";
import { ManageBillingButton } from "../billing/manage-billing-button";

export default function BillingFrameContent() {
	const dataProvider = useCloudProjectDataProvider();

	const [
		{ data: project },
		{
			data: { billing },
		},
	] = useSuspenseQueries({
		queries: [
			dataProvider.currentProjectQueryOptions(),
			dataProvider.currentProjectBillingDetailsQueryOptions(),
		],
	});

	return (
		<>
			<Frame.Header>
				<Frame.Title>{project.displayName} billing</Frame.Title>
				<Frame.Description>
					Manage billing for your Rivet Cloud project.{" "}
					<Link asChild className="cursor-pointer">
						<a
							target="_blank"
							rel="noopener noreferrer"
							href="https://www.rivet.dev/pricing"
						>
							Learn more about billing.
						</a>
					</Link>
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<div className="flex justify-between items-center border rounded-md p-4">
					<div>
						<BillingStatus />
					</div>

					<ManageBillingButton
						variant={
							billing?.canChangePlan ? "secondary" : "default"
						}
					>
						Manage billing details
					</ManageBillingButton>
				</div>
				<BillingPlans />
			</Frame.Content>
		</>
	);
}

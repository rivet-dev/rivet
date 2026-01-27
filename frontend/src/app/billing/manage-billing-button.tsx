import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";

export function ManageBillingButton({
	...props
}: React.ComponentProps<typeof Button>) {
	const dataProvider = useCloudProjectDataProvider();
	const { data, refetch } = useQuery(
		dataProvider.billingCustomerPortalSessionQueryOptions(),
	);

	return (
		<Button
			{...props}
			onMouseEnter={() => {
				refetch();
			}}
			onClick={() => {
				if (data) {
					window.open(data, "_blank");
				}
			}}
		/>
	);
}

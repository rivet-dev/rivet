import { Flex, H1 } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { projectQueryOptions } from "../../queries";

interface BillingHeaderProps {
	projectId: string;
	lead?: ReactNode;
	actions?: ReactNode;
}

export function BillingHeader({
	projectId,
	actions,
	lead,
}: BillingHeaderProps) {
	const {
		data: { displayName },
	} = useSuspenseQuery(projectQueryOptions(projectId));

	return (
		<Flex
			direction={{ initial: "col", md: "row" }}
			gap="4"
			justify="between"
		>
			<Flex direction="col" gap="2">
				<H1>{displayName} Billing</H1>
				{lead}
			</Flex>
			{actions ? <Flex gap="2">{actions}</Flex> : null}
		</Flex>
	);
}

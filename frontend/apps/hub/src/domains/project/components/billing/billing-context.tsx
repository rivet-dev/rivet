import type { Rivet as RivetEe } from "@rivet-gg/api-ee";
import { useSuspenseQuery } from "@tanstack/react-query";
import { type ReactNode, createContext, useContext } from "react";

import { clusterQueryOptions } from "@/domains/auth/queries/bootstrap";
import type { Rivet } from "@rivet-gg/api";
import { startOfMonth } from "date-fns";
import { calculateUsedCredits } from "../../data/billing-calculate-usage";
import {
	projectBillingQueryOptions,
	projectBillingUsageQueryOptions,
	projectQueryOptions,
} from "../../queries";

interface BillingContextValue {
	project: Rivet.cloud.GameFull;
	activePlan: RivetEe.ee.billing.Plan;
	plan: RivetEe.ee.billing.Plan;
	credits: {
		max: number;
		used: number;
		overage: number;
		remaining: number;
		total: number;
	};
	subscription: RivetEe.ee.billing.Subscription | undefined;
}

export const BillingContext = createContext<BillingContextValue | undefined>(
	undefined,
);

interface BillingSubscriptionProviderProps {
	project: Rivet.cloud.GameFull;
	subscription: RivetEe.ee.billing.Subscription | undefined;
	plan: RivetEe.ee.billing.Plan;
	activePlan: RivetEe.ee.billing.Plan;
	children?: ReactNode;
}

const today = new Date();
const firstDayOfMonth = startOfMonth(today);

function BillingSubscriptionProvider({
	children,
	...rest
}: BillingSubscriptionProviderProps) {
	const {
		project: { gameId, developerGroupId },
		subscription,
		activePlan,
		plan,
	} = rest;
	const { data: usage } = useSuspenseQuery(
		projectBillingUsageQueryOptions({
			projectId: gameId,
			groupId: developerGroupId,
			startTs: subscription?.periodStartTs || firstDayOfMonth,
			endTs: subscription?.periodEndTs || today,
		}),
	);

	const credits = calculateUsedCredits({ usage, plan: activePlan });

	return (
		<BillingContext.Provider value={{ credits, ...rest }}>
			{children}
		</BillingContext.Provider>
	);
}

interface BillingProviderProps {
	projectId: string;
	groupId: string;
	children?: ReactNode;
}
function Content({ projectId, groupId, children }: BillingProviderProps) {
	const { data: project } = useSuspenseQuery(projectQueryOptions(projectId));
	const { data } = useSuspenseQuery(projectBillingQueryOptions(projectId));

	if (data) {
		return (
			<BillingSubscriptionProvider
				subscription={data.subscription}
				project={project}
				{...data}
			>
				{children}
			</BillingSubscriptionProvider>
		);
	}
	return children;
}

export const BillingProvider = ({
	projectId,
	groupId,
	children,
}: BillingProviderProps) => {
	const { data } = useSuspenseQuery(clusterQueryOptions());

	if (data === "oss") {
		return children;
	}

	return (
		<Content projectId={projectId} groupId={groupId}>
			{children}
		</Content>
	);
};

export const useBilling = () => {
	const context = useContext(BillingContext);
	if (!context) {
		throw new Error("useBilling must be used within a BillingProvider");
	}
	return context;
};

export const useOptionalBilling = () => {
	return useContext(BillingContext);
};

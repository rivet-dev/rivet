import type { Rivet as RivetEe } from "@rivet-gg/api-ee";
import { useSuspenseQuery } from "@tanstack/react-query";
import { type ReactNode, createContext, useContext } from "react";

import { projectAggregateBillingQueryOptions } from "../../queries";
import { useProject } from "../../data/project-context";
import { clusterQueryOptions } from "@/domains/auth/queries/bootstrap";

interface BillingContextValue {
	/**
	 * The plan that the user is currently on.
	 * This is the plan directly from Stripe
	 */
	activePlan: RivetEe.ee.billing.Plan;
	/**
	 * The plan the user will be on next month, different from `activePlan` if the user has a subscription that will change next month.
	 */
	plan: RivetEe.ee.billing.Plan;
	credits: {
		max: number;
		used: number;
		overage: number;
		remaining: number;
		total: number;
	};
	subscription: RivetEe.ee.billing.Subscription | undefined;
	group: RivetEe.ee.billing.Group;
}

export const BillingContext = createContext<BillingContextValue | undefined>(
	undefined,
);

interface BillingProviderContentProps {
	projectId: string;
	projectNameId: string;
	groupId: string;
	children?: ReactNode;
}
function Content({
	projectId,
	projectNameId,
	groupId,
	children,
}: BillingProviderContentProps) {
	const { data: billing } = useSuspenseQuery(
		projectAggregateBillingQueryOptions({
			projectId,
			projectNameId,
			groupId,
		}),
	);

	return (
		<BillingContext.Provider value={billing}>
			{children}
		</BillingContext.Provider>
	);
}

interface BillingProviderProps {
	children?: ReactNode;
}

export const BillingProvider = ({ children }: BillingProviderProps) => {
	const {
		gameId: projectId,
		nameId,
		developer: { groupId },
	} = useProject();

	const { data } = useSuspenseQuery(clusterQueryOptions());

	if (data === "oss") {
		return children;
	}

	return (
		<Content projectId={projectId} projectNameId={nameId} groupId={groupId}>
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

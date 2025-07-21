import { Icon, faArrowRight } from "@rivet-gg/icons";
import { ActionCard, type ActionCardProps } from "./action-card";
import type { PropsWithChildren } from "react";

export interface CtaCardProps extends Omit<ActionCardProps, "action"> {}

export const CtaCard = (props: CtaCardProps) => {
	return (
		<ActionCard
			{...props}
			action={<Icon icon={faArrowRight} className="size-4" />}
		/>
	);
};

export const CardGroup = ({ children }: PropsWithChildren) => {
	return (
		<div className="not-prose grid gap-4 md:grid-cols-2">{children}</div>
	);
};

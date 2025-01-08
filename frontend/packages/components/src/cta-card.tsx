import { Icon, faArrowRight } from "@rivet-gg/icons";
import { ActionCard, type ActionCardProps } from "./action-card";

interface CtaCardProps extends Omit<ActionCardProps, "action"> {}

export const CtaCard = (props: CtaCardProps) => {
	return (
		<ActionCard
			{...props}
			action={<Icon icon={faArrowRight} className="size-4" />}
		/>
	);
};

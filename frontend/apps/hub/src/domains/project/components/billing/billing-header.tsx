import { H1 } from "@rivet-gg/components";
import type { ReactNode } from "react";

interface BillingHeaderProps {
	lead?: ReactNode;
	actions?: ReactNode;
}

export function BillingHeader({ actions, lead }: BillingHeaderProps) {
	return (
		<>
			<div className="mx-auto my-8 flex justify-between items-center w-full">
				<div className="flex items-center gap-2">
					<H1>Billing</H1>
					{lead}
				</div>
				<div className="flex items-center gap-2">{actions}</div>
			</div>
			<hr className="mb-4" />
		</>
	);
}

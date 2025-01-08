import { Card } from "@rivet-gg/components";
import type { ComponentProps } from "react";

export const AuthCard = (props: ComponentProps<typeof Card>) => (
	// 18.75rem (300px, size of the cf widget) + 1.5rem (24px, padding) + 1.5rem (24px, padding)
	<Card {...props} className="max-w-[21.75rem]" />
);

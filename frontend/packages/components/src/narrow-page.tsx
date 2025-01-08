import { Slot } from "@radix-ui/react-slot";
import type { ReactNode } from "react";
import { Page, type PageProps } from "./page";

export interface NarrowPageProps extends PageProps {
	title?: string;
	children: ReactNode;
}

export const NarrowPage = (props: NarrowPageProps) => {
	return (
		<Slot className="max-w-screen-sm mx-auto">
			<Page {...props} />
		</Slot>
	);
};

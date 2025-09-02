import type { ReactNode } from "react";

interface PageCenteredProps {
	children: ReactNode;
}

const PageCentered = ({ children }: PageCenteredProps) => (
	<div className="flex flex-1 items-center justify-center">
		<div className="max-w-sm">{children}</div>
	</div>
);

export { PageCentered as Root };

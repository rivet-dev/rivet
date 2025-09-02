import { memo, type ReactNode, useState } from "react";
import { cn, ls } from "../lib/utils";
import { ActorsLayoutContextProvider } from "./actors-layout-context";

interface ActorsListPreviewProps {
	left: ReactNode;
	right: ReactNode;
	className?: string;
}

export const ActorsLayout = memo(
	({ left, right, className }: ActorsListPreviewProps) => {
		const [folded, setFolded] = useState(() => ls.actorsList.getFolded());

		return (
			<ActorsLayoutContextProvider
				isFolded={folded}
				setFolded={setFolded}
			>
				<div
					className={cn(
						"w-full relative flex flex-row h-full flex-1 max-h-full min-h-0 overflow-hidden",
						className,
					)}
				>
					{left}
					<div className="h-full max-h-full overflow-hidden flex flex-col flex-grow">
						{right}
					</div>
				</div>
			</ActorsLayoutContextProvider>
		);
	},
);

import { Button, cn } from "@rivet-gg/components";
import {
	Icon,
	faArrowPointer,
	faDiscord,
	faSparkles,
	faWifiSlash,
} from "@rivet-gg/icons";
import type { ComponentProps, ReactNode } from "react";

import { faComment } from "@fortawesome/free-solid-svg-icons/faComment";
import Link from "next/link";

export const HeaderPopupSolutionsMenu = () => {
	return (
		<div className="grid h-full grid-cols-3 grid-rows-2 gap-4 overflow-hidden pb-2">
			<Button
				variant="secondary"
				asChild
				className="h-full justify-start"
				startIcon={<Icon icon={faArrowPointer} />}
			>
				<Link href="/examples" target="_blank">
					Cursors
				</Link>
			</Button>
			<Button
				variant="secondary"
				className="h-full justify-start"
				startIcon={<Icon icon={faComment} />}
			>
				<Link href="/examples" target="_blank">
					Chat App
				</Link>
			</Button>
			<Button
				variant="secondary"
				className="h-full justify-start"
				target="_blank"
				startIcon={<Icon icon={faWifiSlash} />}
			>
				<Link href="/examples" target="_blank">
					Local-first Sync
				</Link>
			</Button>
			<Button
				variant="secondary"
				className="h-full justify-start"
				target="_blank"
				startIcon={<Icon icon={faSparkles} />}
			>
				<Link href="/examples" target="_blank">
					AI Agent
				</Link>
			</Button>
			<Button
				variant="secondary"
				className="h-full justify-start"
				target="_blank"
				startIcon={<Icon icon={faDiscord} />}
			>
				<Link href="/examples" target="_blank">
					Discord Activities
				</Link>
			</Button>
		</div>
	);
};

interface ItemProps extends ComponentProps<"div"> {
	className?: string;
	children?: ReactNode;
}
function Item({ className, children, ...props }: ItemProps) {
	return (
		<div
			className={cn(
				"group h-full cursor-pointer overflow-hidden rounded-md p-4 text-sm grayscale transition-all hover:grayscale-0",
				className,
			)}
			{...props}
		>
			{children}
		</div>
	);
}

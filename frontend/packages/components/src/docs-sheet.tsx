import { Icon, faExternalLink } from "@rivet-gg/icons";
import type { ReactNode } from "react";
import {
	Sheet,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
	SheetTrigger,
} from "./ui/sheet";
import { WithTooltip } from "./ui/tooltip";
import { Link, Text } from "./ui/typography";

interface DocsSheetProps {
	path: string;
	title: string;
	hash?: string;
	children?: ReactNode;
	showTooltip?: boolean;
}

export function DocsSheet({
	path,
	showTooltip,
	title,
	children,
	hash,
}: DocsSheetProps) {
	return (
		<Sheet>
			{showTooltip ? (
				<WithTooltip
					content="Documentation"
					trigger={<SheetTrigger asChild>{children}</SheetTrigger>}
				/>
			) : (
				<SheetTrigger asChild>{children}</SheetTrigger>
			)}
			<SheetContent className="sm:max-w-[500px]">
				<SheetHeader>
					<SheetTitle>{title}</SheetTitle>
					<Text className="text-xs">
						<Link
							href={`https://rivet.gg/${path}?utm_source=hub&embed=true#${hash}`}
							target="_blank"
							rel="noopener noreferrer"
						>
							Open in New Tab <Icon icon={faExternalLink} />
						</Link>
					</Text>
					<SheetDescription className="-mx-6" asChild>
						<div>
							<iframe
								className="w-full h-screen border-t"
								src={`https://rivet.gg/${path}?embed=true#${hash}`}
								title={title}
							/>
						</div>
					</SheetDescription>
				</SheetHeader>
			</SheetContent>
		</Sheet>
	);
}

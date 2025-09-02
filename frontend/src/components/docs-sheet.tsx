import { faExternalLink, Icon } from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { Button } from "./ui/button";
import {
	Sheet,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
	SheetTrigger,
} from "./ui/sheet";
import { WithTooltip } from "./ui/tooltip";

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
					<SheetTitle className="flex items-center justify-between">
						{title}{" "}
						<Button
							asChild
							variant="ghost"
							size="sm"
							className="mr-4"
							endIcon={<Icon icon={faExternalLink} />}
						>
							<a
								target="_blank"
								rel="noopener noreferrer"
								href={
									path.startsWith("http")
										? path
										: `https://rivet.gg/${path}?utm_source=engine&embed=true#${hash}`
								}
							>
								Open in new tab
							</a>
						</Button>
					</SheetTitle>

					<SheetDescription className="-mx-3 mt-4" asChild>
						<div>
							<iframe
								className="w-full h-screen border-t"
								src={
									path.startsWith("http")
										? path
										: `https://rivet.gg/${path}?embed=true#${hash}`
								}
								title={title}
							/>
						</div>
					</SheetDescription>
				</SheetHeader>
			</SheetContent>
		</Sheet>
	);
}

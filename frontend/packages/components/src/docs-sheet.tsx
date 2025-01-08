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
import { Link, Text } from "./ui/typography";

interface DocsSheetProps {
	path: string;
	title: string;
	children?: ReactNode;
}

export function DocsSheet({ path, title, children }: DocsSheetProps) {
	return (
		<Sheet>
			<SheetTrigger asChild>{children}</SheetTrigger>
			<SheetContent className="sm:max-w-[500px]">
				<SheetHeader>
					<SheetTitle>{title}</SheetTitle>
					<Text className="text-xs">
						<Link
							href={`https://rivet.gg/${path}?utm_source=hub`}
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
								src={`https://rivet.gg/${path}?embed=true`}
								title={title}
							/>
						</div>
					</SheetDescription>
				</SheetHeader>
			</SheetContent>
		</Sheet>
	);
}

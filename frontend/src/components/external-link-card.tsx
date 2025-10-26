import { faChevronRight, Icon, type IconProp } from "@rivet-gg/icons";

interface ExternalLinkCardProps {
	href: string;
	icon: IconProp;
	title: string;
	description?: string;
}

export function ExternalLinkCard({
	href,
	icon,
	title,
	description = "Opens in a new tab",
}: ExternalLinkCardProps) {
	return (
		<a href={href} target="_blank" rel="noreferrer" className="block">
			<div className="border rounded-lg p-4 hover:border-primary transition-colors cursor-pointer">
				<div className="flex items-center justify-between">
					<div className="flex items-center gap-3">
						<Icon icon={icon} className="text-2xl" />
						<div>
							<div className="font-medium">{title}</div>
							<div className="text-sm text-muted-foreground">
								{description}
							</div>
						</div>
					</div>
					<Icon
						icon={faChevronRight}
						className="text-muted-foreground"
					/>
				</div>
			</div>
		</a>
	);
}

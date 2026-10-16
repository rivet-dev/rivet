import { faCopy, Icon, type IconProp } from "@rivet-gg/icons";
import { CopyTrigger } from "@/components/copy-area";

/** Icon + title + one-line detail above a connection value. */
export function EndpointTitle({
	icon,
	title,
	detail,
}: {
	icon: IconProp;
	title: string;
	detail?: string;
}) {
	return (
		<div className="flex flex-col gap-0.5">
			<span className="flex items-center gap-2 text-sm font-semibold">
				<Icon icon={icon} className="text-muted-foreground" />
				{title}
			</span>
			{detail ? (
				<span className="text-xs text-muted-foreground">{detail}</span>
			) : null}
		</div>
	);
}

/** A titled URL that copies on click. Shared by the overview and product quickstarts. */
export function Endpoint({
	icon,
	title,
	detail,
	url,
}: {
	icon: IconProp;
	title: string;
	detail?: string;
	url: string;
}) {
	return (
		<div className="flex flex-col gap-2">
			<EndpointTitle icon={icon} title={title} detail={detail} />
			<CopyTrigger value={url}>
				<button
					type="button"
					className="flex items-center justify-between gap-2 rounded-lg border border-border bg-muted/40 px-3 py-2 text-left font-mono text-xs hover:border-muted-foreground/40"
				>
					<span className="truncate">{url}</span>
					<Icon icon={faCopy} className="text-muted-foreground" />
				</button>
			</CopyTrigger>
		</div>
	);
}

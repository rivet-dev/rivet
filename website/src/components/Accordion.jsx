import { Icon, faCaretRight } from "@rivet-gg/icons";
import { clsx } from "clsx";

function getAccordionStyleFromVariant(variant) {
	if (variant === "minimalist") {
		// Minimal padding. Shows a border along the left when open.
		return {
			parentClass: "",
			coverClass: "[&>div]:ml-2 py-1 text-ink-soft hover:text-ink",
			contentClass: "mt-2 pt-1 mb-4 mx-[7px] px-4 border-l border-ink/10",
		};
	}

	// Rounding is handled in Accordion by passing in isRounded to AccordionCover.
	return {
		parentClass: "border border-ink/10 rounded-xl mb-3 overflow-hidden",
		coverClass:
			"py-4 px-5 space-x-2 hover:bg-ink/[0.04] rounded-t-xl transition",
		contentClass: "mt-2 mb-4 mx-6",
	};
}

export function Accordion({
	title,
	description,
	defaultOpen = false,
	icon,
	variant = "rounded",
	children,
}) {
	const { parentClass, coverClass, contentClass } =
		getAccordionStyleFromVariant(variant);

	return (
		<div
			role="listitem"
			className={parentClass}
			data-accordion-container
			data-accordion-open={defaultOpen ? "true" : "false"}
		>
			<button
				type="button"
				data-accordion-trigger
				className={clsx(
					"not-prose flex w-full flex-row content-center items-center",
					coverClass,
				)}
				aria-controls={title + "Children"}
				aria-expanded={defaultOpen}
			>
				<div className="mr-0.5">
					<Icon
						icon={faCaretRight}
						data-accordion-icon
						className={clsx(
							"h-3 w-3 text-ink-faint opacity-75 transition-transform duration-200",
							defaultOpen && "rotate-90"
						)}
					/>
				</div>
				{icon ? (
					<div className="h-4 w-4 fill-ink text-ink">
						{icon}
					</div>
				) : null}
				<div className="text-left leading-tight">
					<p className="m-0 font-medium text-ink">
						{title}
					</p>
					{description ? (
						<p className="m-0 text-ink-soft">
							{description}
						</p>
					) : null}
				</div>
			</button>
			<div
				data-accordion-content
				className={clsx(contentClass, !defaultOpen && "hidden")}
			>
				{children}
			</div>
		</div>
	);
}

export function AccordionGroup({ children }) {
	// [&>div] modifies the Accordion's borders to only show divider borders.
	// We use border-0 instead of border-none because border-none turns off divide-y.
	// [&>div>button] modifies the button to not round the highlighted part
	// when inside of an Accordion group.
	return (
		<div
			className="mb-3 mt-0 divide-y divide-ink/10 overflow-hidden rounded-xl border border-ink/10 [&>div>button]:rounded-none [&>div]:mb-0 [&>div]:rounded-none [&>div]:border-0"
			role="list"
		>
			{children}
		</div>
	);
}

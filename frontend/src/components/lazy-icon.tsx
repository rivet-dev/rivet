import { faActorsBorderless, Icon, type IconProp } from "@rivet-gg/icons";
import {
	type LazyExoticComponent,
	lazy,
	type ReactNode,
	Suspense,
} from "react";
import { cn } from "./lib/utils";

// Each FontAwesome icon ships as its own module so it can be code-split and
// loaded on demand by export name. The glob is resolved relative to this file.
const iconModules = import.meta.glob<Record<string, IconProp>>(
	"../../packages/icons/dist/icons/*.js",
);

const emojiRegex =
	/[\u{1F300}-\u{1F9FF}]|[\u{2600}-\u{26FF}]|[\u{2700}-\u{27BF}]|[\u{1F600}-\u{1F64F}]|[\u{1F680}-\u{1F6FF}]|[\u{1F1E0}-\u{1F1FF}]/u;

export function isEmoji(str: string): boolean {
	return emojiRegex.test(str);
}

// Convert a kebab-case icon name ("arrow-right") to its FontAwesome export name
// ("faArrowRight").
export function toIconExportName(name: string): string {
	return `fa${name
		.split("-")
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join("")}`;
}

type LazyIconComponent = LazyExoticComponent<
	(props: { className?: string; fallback: IconProp }) => ReactNode
>;

const lazyIconCache = new Map<string, LazyIconComponent>();

// The lazy component must be memoized per export name. Creating a fresh `lazy()`
// on every render gives it a new identity, so React remounts it and the
// surrounding Suspense boundary drops back to its loading fallback on every
// parent re-render.
function getLazyIcon(exportName: string): LazyIconComponent {
	const cached = lazyIconCache.get(exportName);
	if (cached) return cached;

	const loader =
		iconModules[`../../packages/icons/dist/icons/${exportName}.js`];
	const component = lazy(() =>
		(loader ? loader() : Promise.reject())
			.then((mod) => ({
				default: ({
					className,
					fallback,
				}: {
					className?: string;
					fallback: IconProp;
				}) => (
					<Icon
						icon={mod[exportName] ?? fallback}
						className={className}
					/>
				),
			}))
			.catch(() => ({
				default: ({
					className,
					fallback,
				}: {
					className?: string;
					fallback: IconProp;
				}) => <Icon icon={fallback} className={className} />,
			})),
	);
	lazyIconCache.set(exportName, component);
	return component;
}

// Lazily renders a FontAwesome icon by its kebab-case name. Suspends while the
// icon module loads, so callers must provide their own Suspense boundary.
export function LazyIcon({
	name,
	className,
	fallback,
}: {
	name: string;
	className?: string;
	fallback: IconProp;
}) {
	const Component = getLazyIcon(toIconExportName(name));
	return <Component className={className} fallback={fallback} />;
}

// Renders an actor icon from its metadata value, which may be an emoji, a
// kebab-case FontAwesome icon name, or absent. Emoji and absent values render
// synchronously; named icons are lazily loaded behind a pulsing fallback.
export function ActorIcon({
	iconValue,
	className,
	emojiClassName,
	fallback = faActorsBorderless,
}: {
	iconValue: string | null;
	className?: string;
	emojiClassName?: string;
	fallback?: IconProp;
}) {
	if (iconValue && isEmoji(iconValue)) {
		return <span className={emojiClassName ?? className}>{iconValue}</span>;
	}

	if (!iconValue) {
		return <Icon icon={fallback} className={className} />;
	}

	return (
		<Suspense
			fallback={
				<Icon
					icon={fallback}
					className={cn(className, "animate-pulse")}
				/>
			}
		>
			<LazyIcon
				name={iconValue}
				className={className}
				fallback={fallback}
			/>
		</Suspense>
	);
}

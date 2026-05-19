import { AssetImage } from "./asset-image";

export function FullscreenLoading({
	children,
}: {
	children?: React.ReactNode;
}) {
	return (
		<div className="min-h-screen flex items-center justify-center flex-col bg-background text-foreground">
			<AssetImage
				className="animate-pulse h-10 invert dark:invert-0"
				src="/logo/icon-white.svg"
			/>
			{children}
		</div>
	);
}

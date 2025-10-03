import { AssetImage } from "./asset-image";

export function FullscreenLoading({
	children,
}: {
	children?: React.ReactNode;
}) {
	return (
		<div className="min-h-screen flex items-center justify-center flex-col">
			<AssetImage
				className="animate-pulse h-10"
				src="/logo/icon-white.svg"
			/>
			{children}
		</div>
	);
}

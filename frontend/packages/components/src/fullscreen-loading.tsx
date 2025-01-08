import { AssetImage } from "./asset-image";

export function FullscreenLoading() {
	return (
		<div className="min-h-screen flex items-center justify-center">
			<AssetImage
				className="animate-pulse h-10"
				src="/logo/icon-white.svg"
			/>
		</div>
	);
}

interface IconWithSpotlightProps {
	iconPath: string;
	title: string;
}

export function IconWithSpotlight({ iconPath, title }: IconWithSpotlightProps) {
	const getPathData = () => {
		switch (iconPath) {
			case "/icons/microchip.svg":
				return "M240 88L240 64L192 64L192 128L128 128L128 192L64 192L64 240L128 240L128 296L64 296L64 344L128 344L128 400L64 400L64 448L128 448L128 512L192 512L192 576L240 576L240 512L296 512L296 576L344 576L344 512L400 512L400 576L448 576L448 512L512 512L512 448L576 448L576 400L512 400L512 344L576 344L576 296L512 296L512 240L576 240L576 192L512 192L512 128L448 128L448 64L400 64L400 128L344 128L344 64L296 64L296 128L240 128L240 88zM464 176L464 464L176 464L176 176L464 176zM368 272L368 368L272 368L272 272L368 272zM272 224L224 224L224 416L416 416L416 224L272 224z";
			case "/icons/database.svg":
				return "M144 270L144 341.4L237.8 400L402.3 400L496.1 341.4L496.1 270L416.1 320L224.1 320L144.1 270zM96 240L96 144L224 64L416 64L544 144L544 496L416 576L224 576L96 496L96 240zM496 192L496 170.6L402.2 112L237.7 112L143.9 170.6L143.9 213.4L237.7 272L402.2 272L496 213.4L496 192zM144 469.4L237.8 528L402.3 528L496.1 469.4L496.1 398L416.1 448L224.1 448L144.1 398L144.1 469.4z";
			case "/icons/bolt.svg":
				return "M414.7 48L407.8 54.6L103.8 342.6L60.1 384L247.5 384L193.4 561L183.9 592L225.9 592L232.8 585.4L536.8 297.4L580.5 256L393.1 256L447.2 79L456.7 48L414.7 48zM180.5 336L374.7 152C345.5 247.7 330 298.3 328.3 304L460 304L265.8 488C295 392.4 310.5 341.7 312.2 336L180.4 336z";
			default:
				return "";
		}
	};

	const pathData = getPathData();

	return (
		<div
			className="relative w-32 h-32 icon-spotlight-container"
			style={{
				['--mouse-x' as string]: '50%',
				['--mouse-y' as string]: '50%',
			}}
		>
			<svg viewBox="0 0 640 640" className="w-full h-full">
				{/* Base stroke - always visible, lights up white on card hover */}
				<path
					d={pathData}
					fill="none"
					stroke="rgba(255, 255, 255, 0.4)"
					strokeWidth="4"
					strokeLinejoin="round"
					className="transition-all duration-200 group-hover:stroke-[rgba(255,255,255,0.7)]"
				/>
			</svg>

			{/* Spotlight layer with mask */}
			<div
				className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-200"
				style={{
					maskImage: `radial-gradient(circle 450px at var(--mouse-x) var(--mouse-y), black 0%, black 20%, transparent 70%)`,
					WebkitMaskImage: `radial-gradient(circle 450px at var(--mouse-x) var(--mouse-y), black 0%, black 20%, transparent 70%)`,
				}}
			>
				<svg viewBox="0 0 640 640" className="w-full h-full">
					<path
						d={pathData}
						fill="rgba(255, 255, 255, 0.15)"
						stroke="rgba(255, 255, 255, 0.6)"
						strokeWidth="4"
						strokeLinejoin="round"
					/>
				</svg>
			</div>
		</div>
	);
}

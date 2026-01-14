import textBlack from "@/images/rivet-logos/icon-text-black.svg";
import textWhite from "@/images/rivet-logos/icon-text-white.svg";

export function Logo({ ...props }) {
	return (
		<div {...props}>
			<img src={textBlack.src}
				alt="Rivet"
				className="h-full w-auto dark:hidden"
			/>
			<img src={textWhite.src}
				alt="Rivet"
				className="hidden dark:block h-full w-auto"
			/>
		</div>
	);
}

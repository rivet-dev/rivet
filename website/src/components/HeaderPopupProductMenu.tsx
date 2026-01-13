import { faBriefcase } from "@fortawesome/free-solid-svg-icons/faBriefcase";
import { faCloud } from "@fortawesome/free-solid-svg-icons/faCloud";
import { faCodeBranch } from "@fortawesome/free-solid-svg-icons/faCodeBranch";
import { Button, cn } from "@rivet-gg/components";
import { Icon, faActors } from "@rivet-gg/icons";
import type { ComponentProps, ReactNode } from "react";


export const HeaderPopupProductMenu = () => {
	return (
		<div className="grid h-full grid-cols-3 grid-rows-3 gap-4 overflow-hidden pb-2">
			<a href="/docs" className="col-span-2 row-span-3 ">
				<Item
					onMouseEnter={(e) =>
						e.currentTarget.querySelector("video")?.play()
					}
					onMouseLeave={(e) =>
						e.currentTarget.querySelector("video")?.pause()
					}
				>
					<div className="relative z-10 h-full">
						<p className="text-base font-bold opacity-80 transition-opacity group-hover:opacity-100">
							<Icon icon={faActors} className="mr-1" />
							Actors
						</p>
						<p className="opacity-80 transition-opacity group-hover:opacity-100">
							The easiest way to build & scale realtime
							applications.
						</p>
					</div>
					<video
						className="absolute inset-0 h-full w-full object-cover opacity-60"
						muted
						loop
						playsInline
						disablePictureInPicture
						disableRemotePlayback
					>
						<source
							src="https://assets2.rivet.dev/effects/bg-effect-product-actors.webm?v=2"
							type="video/webm"
						/>
					</video>
				</Item>
			</a>

			<Button
				variant="secondary"
				asChild
				className="col-start-3 h-full justify-start"
				startIcon={<Icon icon={faCodeBranch} />}
			>
				<a href="https://github.com/rivet-dev/rivet" target="_blank">
					Community Edition
				</a>
			</Button>
			<Button
				variant="secondary"
				className="col-start-3 h-full justify-start"
				startIcon={<Icon icon={faCloud} />}
			>
				<a href="https://dashboard.rivet.dev" target="_blank">
					Rivet Cloud
				</a>
			</Button>
			<Button
				variant="secondary"
				className="col-start-3 h-full justify-start"
				target="_blank"
				startIcon={<Icon icon={faBriefcase} />}
			>
				<a href="/sales">Rivet Enterprise</a>
			</Button>
		</div>
	);
};

interface ItemProps extends ComponentProps<"div"> {
	className?: string;
	children?: ReactNode;
}
function Item({ className, children, ...props }: ItemProps) {
	return (
		<div
			className={cn(
				"group h-full cursor-pointer overflow-hidden rounded-md p-4 text-sm grayscale transition-all hover:grayscale-0",
				className,
			)}
			{...props}
		>
			{children}
		</div>
	);
}

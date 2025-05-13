import { cn, ls } from "../lib/utils";
import { Icon, faGripDotsVertical } from "@rivet-gg/icons";
import {
	animate,
	motion,
	useMotionTemplate,
	useMotionValue,
	useMotionValueEvent,
	useTransform,
} from "framer-motion";
import {
	type ReactNode,
	Suspense,
	memo,
	useCallback,
	useLayoutEffect,
	useState,
} from "react";
import { ActorsLayoutContextProvider } from "./actors-layout-context";
import { ActorsListPanel } from "./actors-list-panel";

const RIGHT_PANEL_MIN_WIDTH = 480;

interface ActorsListPreviewProps {
	children: ReactNode;
}

export const ActorsListPreview = memo(
	({ children }: ActorsListPreviewProps) => {
		const outerWidth = useMotionValue(0);
		const x = useMotionValue(0);

		const rightWidth = useMotionTemplate`calc(50% - ${x}px)`;
		const leftWidth = useMotionTemplate`calc(50% + ${x}px)`;

		const [folded, setFolded] = useState(() => ls.actorsList.getFolded());
		const [isDragging, setIsDragging] = useState(false);

		const [, setInitialized] = useState(false);

		useMotionValueEvent(x, "change", (value) => {
			ls.actorsList.set(value / outerWidth.get(), folded);
		});

		// biome-ignore lint/correctness/useExhaustiveDependencies: on first draw
		useLayoutEffect(() => {
			x.setCurrent((ls.actorsList.getWidth() || 0) * outerWidth.get());
			setInitialized(true);
		}, []);

		const relativeOffset = useTransform(
			[x, outerWidth],
			([value, outerWidth]: number[]) => {
				const center = outerWidth / 2;
				const percent = (value + center) / outerWidth;
				// 0.5 is the center
				// 0 is the left
				// 1 is the right
				return percent;
			},
		);

		const opacity = useTransform(relativeOffset, [0, 0.1], [0, 1]);
		const pointerEvents = useTransform(opacity, () => {
			return opacity.get() > 0.5 ? "auto" : "none";
		});

		const toggle = useCallback(
			(newValue: boolean) => {
				setFolded(newValue);
				if (newValue) {
					animate(x, -outerWidth.get() / 2);
				} else {
					animate(x, 0);
				}
			},
			[outerWidth, x],
		);

		return (
			<ActorsLayoutContextProvider isFolded={folded} setFolded={toggle}>
				<div
					className="w-full relative flex flex-row h-full flex-1 max-h-full min-h-0 overflow-hidden"
					ref={(ref) => {
						if (ref) {
							const width = ref.getBoundingClientRect().width;
							outerWidth.set(width);
						}
					}}
				>
					<motion.div
						className="h-full min-h-0 flex overflow-hidden"
						style={{
							width: leftWidth,
						}}
						animate={{
							opacity: folded ? 0 : 1,
						}}
					>
						<ActorsListPanel />
					</motion.div>
					<motion.div
						drag="x"
						_dragX={x}
						style={{ opacity, pointerEvents }}
						onDragStart={() => setIsDragging(true)}
						onDrag={(e, info) => {
							const rightPos = outerWidth.get() - info.point.x;
							setFolded(outerWidth.get() - rightPos < 470);
						}}
						onDoubleClick={() => {
							setFolded(!folded);
							if (folded) {
								animate(x, 0);
							} else {
								animate(x, -outerWidth.get() / 2);
							}
						}}
						onDragEnd={(e, info) => {
							if (folded) {
								animate(x, -outerWidth.get() / 2);
							} else {
								const leftPos = outerWidth.get() - info.point.x;
								if (leftPos < RIGHT_PANEL_MIN_WIDTH) {
									animate(
										x,
										outerWidth.get() / 2 -
											RIGHT_PANEL_MIN_WIDTH,
									);
								}
							}
							setIsDragging(false);
						}}
						dragMomentum={false}
						className="w-[1px] bg-border cursor-col-resize inset-y-0 z-20 relative flex items-center group"
					>
						<Icon
							icon={faGripDotsVertical}
							className={cn(
								"w-4 h-4 -translate-x-1/2 group-hover:opacity-100 opacity-10 transition-all scale-100 p-6",
								isDragging && "opacity-100 scale-125",
							)}
						/>
					</motion.div>
					<motion.div
						className="h-full max-h-full overflow-hidden flex flex-col flex-grow"
						style={{
							width: rightWidth,
							minWidth: RIGHT_PANEL_MIN_WIDTH,
						}}
					>
						<Suspense>{children}</Suspense>
					</motion.div>
				</div>
			</ActorsLayoutContextProvider>
		);
	},
);

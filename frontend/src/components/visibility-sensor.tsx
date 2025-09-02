import { useEffect, useRef } from "react";

export function VisibilitySensor({
	onChange,
	onToggle,
	className,
}: {
	onChange?: () => void;
	onToggle?: (isVisible: boolean) => void;
	className?: string;
}) {
	const ref = useRef<HTMLDivElement>(null);

	useEffect(() => {
		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0].isIntersecting) {
					onChange?.();
					onToggle?.(true);
				}
			},
			{ threshold: 1.0 },
		);
		if (ref.current) {
			observer.observe(ref.current);
		}
		return () => {
			if (ref.current) {
				observer.unobserve(ref.current);
			}
		};
	}, [onChange, onToggle]);

	return <div ref={ref} className={className} style={{ height: "1px" }} />;
}

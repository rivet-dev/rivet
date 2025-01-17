"use client";

import grainDark from "@/images/effects/grain-dark.png";
import clsx from "clsx";
import Link from "next/link";

export function PatternButton({ children, highlight, ...props }) {
	const Component = props.href ? Link : "button";

	return (
		<Component
			{...props}
			highlight={highlight}
			className={clsx(
				"group relative transition",
				"flex",
				"text-sm font-semibold text-cream-100",
				"border-2",
				"bg-charcole-950",
				highlight
					? "border-cream-100"
					: "border-cream-100/50 hover:border-orange-400",
				props.className,
			)}
		>
			{/* Background */}
			<div
				style={{
					backgroundImage: `url(${grainDark.src})`,
					opacity: highlight ? 1 : 0,
				}}
				className="pointer-events-none absolute inset-0 bg-repeat transition group-hover:opacity-100"
			></div>
			{/* <div style={{ backgroundImage: `url(${grainLight.src})`, zIndex: -1, opacity: hover ? 1 : 0 }} className='absolute inset-0 bg-repeat transition pointer-events-none'></div> */}

			{/* Children */}
			<div
				className={clsx(
					"z-10 h-full w-full",
					highlight
						? "opacity-100"
						: "opacity-75 group-hover:opacity-100",
				)}
			>
				{children}
			</div>
		</Component>
	);
}

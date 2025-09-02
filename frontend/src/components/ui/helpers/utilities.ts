import type { Responsive } from "./types";

export function getResponsiveValue<T extends string>(
	value: Responsive<T> | undefined,
	key: string,
	{ useDash }: { useDash?: boolean } = { useDash: true },
) {
	const separator = useDash ? "-" : "";

	if (typeof value === "object") {
		return Object.entries(value)
			.map(([breakpoint, value]) => {
				if (breakpoint === "initial") {
					return `${key}${separator}${value}`;
				}
				if (value.startsWith("-")) {
					return `${breakpoint}:-${key}${separator}${value.replace("-", "")}`;
				}
				return `${breakpoint}:${key}${separator}${value}`;
			})
			.join(" ");
	}

	if (value?.startsWith("-")) {
		return `-${key}${separator}${value.replace("-", "")}`;
	}

	return `${key}${separator}${value}`;
}

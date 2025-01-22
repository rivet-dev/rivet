import resolveConfig from "tailwindcss/resolveConfig";
import { useMediaQuery } from "usehooks-ts";
import config from "../tailwind-base";

const resolvedConfig = resolveConfig(config);

const breakpoints = resolvedConfig.theme.screens;

type BreakpointKey = keyof typeof breakpoints;

export function useBreakpoint<Point extends BreakpointKey>(point: Point) {
	return useMediaQuery(`(min-width: ${breakpoints[point]})`);
}

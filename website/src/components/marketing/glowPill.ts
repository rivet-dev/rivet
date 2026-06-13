import type { MouseEvent } from 'react';

// Structural classes a porcelain pill needs for the cursor-following glow:
// `relative` anchors the ::after, `overflow-hidden` clips the blurred gradient
// to the rounded shape, and `glow-pill` owns the ::after (see main.css). Append
// to a pill's existing className.
export const GLOW_PILL_CLASS = 'glow-pill relative overflow-hidden';

// Feeds the pointer position (relative to the hovered pill) into the
// --pill-x / --pill-y custom properties the .glow-pill ::after reads. Mirrors
// the changelog pill handler; no rAF needed since it only fires on the small
// hovered element.
export const handleGlowPillMouseMove = (event: MouseEvent<HTMLElement>) => {
	const rect = event.currentTarget.getBoundingClientRect();
	event.currentTarget.style.setProperty('--pill-x', `${event.clientX - rect.left}px`);
	event.currentTarget.style.setProperty('--pill-y', `${event.clientY - rect.top}px`);
};

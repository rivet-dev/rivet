import type { MouseEvent } from 'react';

// Structural classes a porcelain pill needs for the cursor-following border
// glow: `relative` anchors the ::after, and `glow-pill` owns the ::after (see
// main.css) which lights ONLY the pill's border ring as the cursor moves. No
// `overflow-hidden` here — it would clip the border-aligned ring. Append to a
// pill's existing className.
export const GLOW_PILL_CLASS = 'glow-pill relative';

// Feeds the pointer position (relative to the hovered pill) into the
// --pill-x / --pill-y custom properties the .glow-pill ::after reads. Mirrors
// the changelog pill handler; no rAF needed since it only fires on the small
// hovered element.
export const handleGlowPillMouseMove = (event: MouseEvent<HTMLElement>) => {
	const rect = event.currentTarget.getBoundingClientRect();
	event.currentTarget.style.setProperty('--pill-x', `${event.clientX - rect.left}px`);
	event.currentTarget.style.setProperty('--pill-y', `${event.clientY - rect.top}px`);
};

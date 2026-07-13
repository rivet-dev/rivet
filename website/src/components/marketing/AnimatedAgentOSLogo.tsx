'use client';

import { useEffect, useRef, useState } from 'react';

// Animated agentOS wordmark, ported from the agent-os site (AgentOSPage.tsx).
// Fetches the layered wordmark SVG and reveals the glyphs with a pen-draw mask.
// Keep the mask math in sync with the agent-os source when the asset changes.
interface AnimatedAgentOSLogoProps {
	className?: string;
	// Seconds for the main stroke-draw. Defaults to 3; pass a smaller value for
	// a faster intro.
	drawDurationSec?: number;
}

export const AnimatedAgentOSLogo = ({ className, drawDurationSec = 3 }: AnimatedAgentOSLogoProps) => {
	const containerRef = useRef<HTMLDivElement>(null);
	const [isReady, setIsReady] = useState(false);

	useEffect(() => {
		const container = containerRef.current;
		if (!container) return;

		fetch('/images/agent-os/agentos-hero-logo-animated.svg')
			.then((res) => res.text())
			.then((svgText) => {
				container.innerHTML = svgText;

				const svg = container.querySelector('svg');
				if (!svg) return;

				svg.removeAttribute('width');
				svg.removeAttribute('height');
				svg.style.height = '100%';
				svg.style.width = 'auto';
				svg.style.display = 'block';

				const ns = 'http://www.w3.org/2000/svg';
				const textLayer = svg.querySelector('#text-layer');
				const strokeLayer = svg.querySelector('#stroke-layer');
				if (!textLayer || !strokeLayer) return;

				const strokePath = strokeLayer.querySelector('path');
				if (!strokePath) return;

				// The wordmark reveals the real glyphs through a mask that follows
				// the pen path, so the finished logo is the brand mark itself and
				// nothing settles or swaps after the pen stops. Three strokes per
				// subpath keep the reveal honest at self-intersections: a wide white
				// stroke reveals the written portion, a black stroke over the
				// unwritten remainder conceals ink the pen has not reached yet (the
				// hanging edges at crossings), and a narrower white stroke on top
				// restores the written line where the unwritten portion crosses it.
				// pathLength=100 normalizes the dash math so one keyframe pair
				// serves every stroke regardless of geometric length.
				const fullD = strokePath.getAttribute('d') || '';
				const lastM = fullD.lastIndexOf('M');
				const mainD = fullD.substring(0, lastM);
				const tailD = fullD.substring(lastM);

				const defs = document.createElementNS(ns, 'defs');
				svg.insertBefore(defs, svg.firstChild);
				const mask = document.createElementNS(ns, 'mask');
				mask.setAttribute('id', 'agentos-reveal-mask');
				mask.setAttribute('maskUnits', 'userSpaceOnUse');
				mask.setAttribute('x', '0');
				mask.setAttribute('y', '0');
				mask.setAttribute('width', '99999');
				mask.setAttribute('height', '99999');
				const groupTransform = strokeLayer.getAttribute('transform') || '';
				const makeMaskStroke = (d: string, color: string, width: number, cap: string) => {
					const group = document.createElementNS(ns, 'g');
					group.setAttribute('transform', groupTransform);
					const path = document.createElementNS(ns, 'path');
					path.setAttribute('d', d);
					path.setAttribute('pathLength', '100');
					path.setAttribute(
						'style',
						`fill:none; stroke:${color}; stroke-width:${width}px; stroke-linecap:${cap}; stroke-linejoin:round;`,
					);
					// The gap exceeds the dash by 1 so no offset ever lands a
					// zero-length dash on the path end. With round caps that
					// degenerate dash renders as a floating dot.
					path.style.strokeDasharray = '100 101';
					group.appendChild(path);
					mask.appendChild(group);
					return path;
				};
				// Paint order matters: reveal below, conceal above it, restore on top.
				// The conceal stroke uses butt caps so it ends exactly at the pen tip
				// instead of eating half a cap backwards into fresh ink. The restore
				// stroke is only a hair wider than the letterform (~6.8 in this
				// space): any wider and it re-reveals concealed ink where an
				// unwritten stroke crosses a written one.
				const reveals = [makeMaskStroke(mainD, 'white', 10.57, 'round'), makeMaskStroke(tailD, 'white', 10.57, 'round')];
				const conceals = [makeMaskStroke(mainD, 'black', 10.57, 'butt'), makeMaskStroke(tailD, 'black', 10.57, 'butt')];
				const restores = [makeMaskStroke(mainD, 'white', 7.6, 'round'), makeMaskStroke(tailD, 'white', 7.6, 'round')];
				defs.appendChild(mask);

				// Wrap the text layer in a masked group.
				const textParent = textLayer.parentNode;
				if (textParent) {
					const wrapper = document.createElementNS(ns, 'g');
					wrapper.setAttribute('mask', 'url(#agentos-reveal-mask)');
					textParent.insertBefore(wrapper, textLayer);
					wrapper.appendChild(textLayer);
				}
				strokeLayer.remove();

				// Animate like a real pen. The word must keep drawing at near-constant
				// speed to the end: an ease-out crawl leaves the "t" sitting uncrossed
				// and half-drawn for the last second of the animation. The crossbar
				// follows after a brief pen lift as a quick flick.
				const mainDuration = drawDurationSec;
				const penLift = 0.09;
				const tailDuration = 0.16;

				if (!document.querySelector('#agentos-logo-animation-style')) {
					const style = document.createElement('style');
					style.id = 'agentos-logo-animation-style';
					style.textContent = `
						@keyframes agentos-draw {
							to { stroke-dashoffset: 0; }
						}
						@keyframes agentos-conceal {
							to { stroke-dashoffset: -100; }
						}
					`;
					document.head.appendChild(style);
				}

				if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
					// Static: mask fully open, filled glyphs shown immediately.
					for (const path of [...reveals, ...restores]) path.style.strokeDashoffset = '0';
					for (const path of conceals) path.style.strokeDashoffset = '-100';
				} else {
					const timings = [
						`${mainDuration}s cubic-bezier(0.3, 0, 0.75, 0.85) 0s`,
						`${tailDuration}s cubic-bezier(0.3, 0, 0.6, 1) ${mainDuration + penLift}s`,
					];
					[reveals, restores].forEach((pair) =>
						pair.forEach((path, i) => {
							path.style.strokeDashoffset = '100.5';
							path.style.animation = `agentos-draw ${timings[i]} forwards`;
						}),
					);
					conceals.forEach((path, i) => {
						path.style.strokeDashoffset = '0';
						path.style.animation = `agentos-conceal ${timings[i]} forwards`;
					});
				}

				setIsReady(true);
			});

		return () => {
			if (container) {
				container.innerHTML = '';
			}
		};
	}, [drawDurationSec]);

	return (
		<div
			ref={containerRef}
			className={className}
			style={{
				opacity: isReady ? 1 : 0,
				transition: 'opacity 0.3s ease',
			}}
		/>
	);
};

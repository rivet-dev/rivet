'use client';

import { useState, useEffect, useRef, useCallback } from 'react';
import {
	ArrowDown,
	ArrowRight,
	Shield,
	Terminal,
	FolderOpen,
	Clock,
	Layers,
	Globe,
	Bot,
	Wrench,
	CalendarClock,
	ExternalLink,
	Activity,
	HardDrive,
	Code,
	Cpu,
	Package,
	Users,
	Webhook,
	Workflow,
	ChevronLeft,
	ChevronRight,
	Copy,
	Check,
	GitBranch,
	Container,
	ShieldCheck,
} from 'lucide-react';
import { AnimatePresence, motion } from 'framer-motion';
import { Icon, faNodeJs, faPython } from '@rivet-gg/icons';
import type { IconProp } from '@rivet-gg/icons';
import agentosLogo from '@/images/products/agentos-logo.svg';
import { InkPanel } from '../editorial/InkPanel';
import { Eyebrow } from '../editorial/Eyebrow';
import { PixelGridChart } from '../art/PixelGridChart';

interface HeroTabCode {
	key: string;
	fileName: string;
	code: string;
	highlightedCode: string;
}

interface AgentOSPageProps {
	heroTabs: HeroTabCode[];
}

// --- Animated agentOS Logo ---
interface AnimatedAgentOSLogoProps {
	className?: string;
	displayedAgent?: { src: string; name: string } | null;
}

const AnimatedAgentOSLogo = ({ className, displayedAgent }: AnimatedAgentOSLogoProps) => {
	const containerRef = useRef<HTMLDivElement>(null);
	const [isReady, setIsReady] = useState(false);
	const osLayerRef = useRef<Element | null>(null);
	const agentImageRef = useRef<SVGImageElement | null>(null);

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

				// Find and store reference to the OS layer (contains the "OS" text)
				const osLayer = svg.querySelector('#os-layer');
				if (osLayer) {
					osLayerRef.current = osLayer;
					// Set up transition for smooth opacity changes
					(osLayer as HTMLElement).style.transition = 'opacity 0.15s ease-out';
				}

				// Create agent image element inside the os-layer's parent, positioned like os-layer
				// The image will be positioned to appear inside the squircle where "OS" is
				const agentImg = document.createElementNS(ns, 'image');
				agentImg.setAttribute('id', 'agent-logo');
				// Position inside the squircle (viewBox is 0 0 305 102, squircle is on the right)
				agentImg.setAttribute('width', '32');
				agentImg.setAttribute('height', '32');
				agentImg.setAttribute('x', '249');
				agentImg.setAttribute('y', '25');
				agentImg.setAttribute('preserveAspectRatio', 'xMidYMid meet');
				agentImg.style.opacity = '0';
				agentImg.style.transition = 'opacity 0.15s ease-out';
				svg.appendChild(agentImg);
				agentImageRef.current = agentImg;

				const strokePath = strokeLayer.querySelector('path');
				if (!strokePath) return;

				const strokeStyle =
					'fill:none; stroke:white; stroke-width:10.57px; stroke-linecap:round; stroke-linejoin:round;';

				// Split the path data into main path and short tail path
				const fullD = strokePath.getAttribute('d') || '';
				const lastM = fullD.lastIndexOf('M');
				const mainD = fullD.substring(0, lastM);
				const tailD = fullD.substring(lastM);

				// Create mask
				const defs = document.createElementNS(ns, 'defs');
				svg.insertBefore(defs, svg.firstChild);

				const mask = document.createElementNS(ns, 'mask');
				mask.setAttribute('id', 'reveal-mask');
				mask.setAttribute('maskUnits', 'userSpaceOnUse');
				mask.setAttribute('x', '0');
				mask.setAttribute('y', '0');
				mask.setAttribute('width', '99999');
				mask.setAttribute('height', '99999');

				// Clone the stroke group transform wrapper for both paths
				const groupTransform = strokeLayer.getAttribute('transform') || '';

				// Main path
				const mainGroup = document.createElementNS(ns, 'g');
				mainGroup.setAttribute('transform', groupTransform);
				const mainPath = document.createElementNS(ns, 'path');
				mainPath.setAttribute('d', mainD);
				mainPath.setAttribute('style', strokeStyle);
				mainGroup.appendChild(mainPath);
				mask.appendChild(mainGroup);

				// Tail path
				const tailGroup = document.createElementNS(ns, 'g');
				tailGroup.setAttribute('transform', groupTransform);
				const tailPath = document.createElementNS(ns, 'path');
				tailPath.setAttribute('d', tailD);
				tailPath.setAttribute('style', strokeStyle);
				tailGroup.appendChild(tailPath);
				mask.appendChild(tailGroup);

				defs.appendChild(mask);

				// Wrap text layer in a masked group
				const parent = textLayer.parentNode;
				if (parent) {
					const wrapper = document.createElementNS(ns, 'g');
					wrapper.setAttribute('mask', 'url(#reveal-mask)');
					parent.insertBefore(wrapper, textLayer);
					wrapper.appendChild(textLayer);
				}

				// Remove the original stroke layer
				strokeLayer.remove();

				// Measure path lengths
				const mainLength = mainPath.getTotalLength();
				const tailLength = tailPath.getTotalLength();

				// Set up dash offsets (hidden initially)
				mainPath.style.strokeDasharray = String(mainLength);
				mainPath.style.strokeDashoffset = String(mainLength);
				tailPath.style.strokeDasharray = String(tailLength);
				tailPath.style.strokeDashoffset = String(tailLength);

				// Animate: main path first, then tail after main finishes
				const mainDuration = 3;
				const tailDuration = 0.3;

				// Add keyframes if not already present
				if (!document.querySelector('#agentos-logo-animation-style')) {
					const style = document.createElement('style');
					style.id = 'agentos-logo-animation-style';
					style.textContent = `
						@keyframes reveal-main {
							to { stroke-dashoffset: 0; }
						}
						@keyframes reveal-tail {
							to { stroke-dashoffset: 0; }
						}
					`;
					document.head.appendChild(style);
				}

				mainPath.style.animation = `reveal-main ${mainDuration}s ease forwards`;
				tailPath.style.animation = `reveal-tail ${tailDuration}s ease ${mainDuration}s forwards`;

				setIsReady(true);
			});

		return () => {
			if (container) {
				container.innerHTML = '';
			}
		};
	}, []);

	// Update OS layer and agent image visibility when displayedAgent changes
	useEffect(() => {
		if (!isReady) return;

		const osLayer = osLayerRef.current;
		const agentImg = agentImageRef.current;

		if (osLayer && agentImg) {
			if (displayedAgent) {
				// Hide OS layer, show agent logo
				(osLayer as HTMLElement).style.opacity = '0';
				agentImg.setAttributeNS('http://www.w3.org/1999/xlink', 'xlink:href', displayedAgent.src);
				agentImg.setAttribute('href', displayedAgent.src);
				agentImg.style.opacity = '1';
			} else {
				// Show OS layer, hide agent logo
				(osLayer as HTMLElement).style.opacity = '1';
				agentImg.style.opacity = '0';
			}
		}
	}, [displayedAgent, isReady]);

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

// --- Hero Image Data ---
interface HeroImage {
	src: string;
	title: string;
	caption: string;
}

const heroImages: HeroImage[] = [
	// Human work
	{
		src: 'https://assets.rivet.dev/website/public/images/agent-os/division-classification-cataloging.jpg',
		title: 'Division of Classification and Cataloging',
		caption: 'Manual human labor at scale',
	},
	{
		src: 'https://assets.rivet.dev/website/public/images/agent-os/crowded-office-space.jpg',
		title: 'Crowded Office Space',
		caption: 'Rooms full of human operators',
	},
	// Automation with computers
	{
		src: 'https://assets.rivet.dev/website/public/images/agent-os/early-computer-room.jpg',
		title: 'Early Computer Room',
		caption: 'The first machines',
	},
	{
		src: 'https://assets.rivet.dev/website/public/images/agent-os/unix-timesharing-uw-madison-1978.jpg',
		title: 'Unix Timesharing',
		caption: 'UW-Madison, 1978',
	},
	{
		src: 'https://assets.rivet.dev/website/public/images/agent-os/early-computing-workstation.jpg',
		title: 'Early Computing Workstation',
		caption: 'Humans operating computers',
	},
	{
		src: 'https://assets.rivet.dev/website/public/images/agent-os/apollo-14-mission-control.jpg',
		title: 'Apollo 14: Mission Control Center',
		caption: 'Computers in mission-critical work',
	},
	// Modern work
	{
		src: 'https://assets.rivet.dev/website/public/images/agent-os/modern-office.jpg',
		title: 'Modern Office',
		caption: "Today's human operators",
	},
	// AI agents of tomorrow
	{
		src: 'https://assets.rivet.dev/website/public/images/agent-os/data-flock.jpg',
		title: 'Data Flock (digits)',
		caption: 'The agent era',
	},
];

// --- Image Cycler (adapted from landing page) ---
const ImageCycler = ({ images }: { images: HeroImage[] }) => {
	const [currentIndex, setCurrentIndex] = useState(0);
	const [showFan, setShowFan] = useState(false);
	const [leavingCards, setLeavingCards] = useState<Array<{ id: string; image: HeroImage }>>([]);

	useEffect(() => {
		const preloadAhead = Math.min(4, images.length - 1);
		for (let i = 1; i <= preloadAhead; i++) {
			const next = images[(currentIndex + i) % images.length];
			const img = new window.Image();
			img.src = next.src;
		}
	}, [currentIndex, images]);

	const handleClick = () => {
		const leavingImage = images[currentIndex];
		setLeavingCards((prev) => [...prev, { id: `${leavingImage.src}-${Date.now()}`, image: leavingImage }]);
		setCurrentIndex((prev) => (prev + 1) % images.length);
	};

	const getStackIndices = (count: number) => {
		const indices = [];
		for (let i = 0; i < count; i++) {
			indices.push((currentIndex + i) % images.length);
		}
		return indices;
	};

	const getStackPose = (position: number, expanded: boolean) => {
		const basePoses = [
			{ x: 0, y: 0, rotate: -0.7, scale: 1 },
			{ x: 5, y: 2, rotate: 1.2, scale: 0.985 },
			{ x: 10, y: 4, rotate: 2.4, scale: 0.97 },
		];

		const expandedOffsets = [
			{ x: -6, y: 0, rotate: -0.8 },
			{ x: 8, y: -4, rotate: 1.1 },
			{ x: 16, y: -8, rotate: 1.7 },
		];

		const idx = Math.min(position, basePoses.length - 1);
		const base = basePoses[idx];
		const expand = expanded ? expandedOffsets[idx] : { x: 0, y: 0, rotate: 0 };

		if (!expanded) {
			return { x: 0, y: 0, rotate: 0, scale: 1 };
		}

		return {
			x: base.x + expand.x,
			y: base.y + expand.y,
			rotate: base.rotate + expand.rotate,
			scale: base.scale,
		};
	};

	const stackCards = getStackIndices(Math.min(3, images.length));
	const currentImage = images[currentIndex];

	return (
		<div
			className='relative w-[280px] h-[350px] sm:w-[400px] sm:h-[500px] cursor-pointer'
			onClick={handleClick}
			onMouseEnter={() => setShowFan(true)}
			onMouseLeave={() => setShowFan(false)}
		>
			<div
				className={`pointer-events-none absolute -inset-3 rounded-xl bg-black/20 blur-2xl transition-all duration-300 ease-out ${
					showFan ? 'opacity-100 scale-105' : 'opacity-0 scale-100'
				}`}
				style={{ zIndex: 0 }}
			/>

			{stackCards.map((imageIndex, stackPosition) => {
				const pose = getStackPose(stackPosition, showFan);
				const image = images[imageIndex];
				const isTopCard = stackPosition === 0;

				return (
					<motion.div
						key={image.src}
						className={`absolute inset-0 rounded-lg overflow-hidden border ${
							showFan ? 'border-black/20' : 'border-black/0'
						} ${isTopCard ? 'shadow-2xl' : 'shadow-xl'}`}
						style={{
							zIndex: 20 - stackPosition,
							boxShadow: isTopCard && showFan ? '0 28px 70px rgba(0, 0, 0, 0.15)' : undefined,
						}}
						initial={false}
						animate={{ ...pose, opacity: isTopCard || showFan ? 1 : 0 }}
						transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
					>
						<img
							src={image.src}
							alt={image.title}
							loading={isTopCard && currentIndex === 0 ? 'eager' : 'lazy'}
							decoding='async'
							className='w-full h-full object-cover select-none pointer-events-none'
						/>
						{isTopCard ? <div className='absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-transparent' /> : null}
					</motion.div>
				);
			})}

			<AnimatePresence initial={false}>
				{leavingCards.map((card) => {
					const topPose = getStackPose(0, showFan);

					return (
						<motion.div
							key={card.id}
							className={`pointer-events-none absolute inset-0 rounded-lg overflow-hidden border ${
								showFan ? 'border-black/20' : 'border-black/0'
							} shadow-2xl`}
							style={{ zIndex: 30 }}
							initial={{ ...topPose, opacity: 1 }}
							animate={{ x: topPose.x - 36, y: topPose.y - 2, rotate: topPose.rotate - 7, scale: 0.985, opacity: 0 }}
							transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
							onAnimationComplete={() =>
								setLeavingCards((prev) => prev.filter((prevCard) => prevCard.id !== card.id))
							}
						>
							<img
								src={card.image.src}
								alt={card.image.title}
								loading='lazy'
								decoding='async'
								className='w-full h-full object-cover select-none pointer-events-none'
							/>
							<div className='absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-transparent' />
						</motion.div>
					);
				})}
			</AnimatePresence>

			<div
				className={`pointer-events-none absolute left-0 right-0 top-full mt-3 text-center transition-all duration-200 ${
					showFan ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-1'
				}`}
				style={{ zIndex: 20 }}
			>
				<p className='text-sm font-medium text-ink'>{currentImage.title}</p>
				<p className='text-xs text-ink-faint'>{currentImage.caption}</p>
			</div>
		</div>
	);
};

// --- Copy Command ---
const CopyCommand = ({ command }: { command: string }) => {
	const [copied, setCopied] = useState(false);

	const handleCopy = async () => {
		await navigator.clipboard.writeText(command);
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	};

	return (
		<button
			onClick={handleCopy}
			className='group inline-flex w-full items-center justify-center gap-2 whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 text-sm transition-colors hover:border-ink/40 sm:w-auto'
		>
			<Terminal className='h-4 w-4 text-ink-faint' />
			<span className='text-ink-soft transition-colors group-hover:text-ink'>{command}</span>
			{copied && <Check className='h-4 w-4 text-pine' />}
		</button>
	);
};

// --- Handwriting Text ---
const HandwrittenText = ({ text, className }: { text: string; className?: string }) => {
	const textRef = useRef<SVGTextElement>(null);
	const [measured, setMeasured] = useState<{ width: number; height: number } | null>(null);

	useEffect(() => {
		const doMeasure = () => {
			const el = textRef.current;
			if (!el) return;
			const box = el.getBBox();
			if (box.width > 0) {
				setMeasured({ width: box.width + 20, height: box.height + 20 });
			}
		};

		if (document.fonts) {
			document.fonts.ready.then(() => {
				requestAnimationFrame(() => {
					requestAnimationFrame(doMeasure);
				});
			});
		} else {
			setTimeout(doMeasure, 500);
		}
	}, []);

	return (
		<svg
			viewBox={measured ? `0 0 ${measured.width} ${measured.height}` : '0 0 800 120'}
			className={className}
			style={{ overflow: 'visible' }}
			preserveAspectRatio='xMidYMid meet'
		>
			<text
				ref={textRef}
				x='10'
				y={measured ? measured.height * 0.75 : 90}
				style={{
					fontFamily: '"Gloria Hallelujah", cursive',
					fontSize: '72px',
					fontWeight: 400,
					fill: '#1B1916',
					stroke: '#1B1916',
					strokeWidth: 1,
					paintOrder: 'stroke fill',
				}}
			>
				{text}
			</text>
		</svg>
	);
};

// --- Fake Terminal ---
const AGENTOS_ASCII = `      db                                  mm     .g8""8q.    .M"""bgd
     ;MM:                                 MM   .dP'    \`YM. ,MI    "Y
    ,V^MM.    .P"Ybmmm .gP"Ya \`7MMpMMMb.mmMMmm dM'      \`MM \`MMb.
   ,M  \`MM   :MI  I8  ,M'   Yb  MM    MM  MM   MM        MM   \`YMMNq.
   AbmmmqMA   WmmmP"  8M""""""  MM    MM  MM   MM.      ,MP .     \`MM
  A'     VML 8M       YM.    ,  MM    MM  MM   \`Mb.    ,dP' Mb     dM
.AMA.   .AMMA.YMMMMMb  \`Mbmmd'.JMML  JMML.\`Mbmo  \`"bmmd"'   P"Ybmmd"
             6'     dP
             Ybmmmd'`;

interface TermLine {
	text: string;
	color?: string;
	delay: number;
	typing?: boolean;
}

const terminalLines: TermLine[] = [
	{ text: '$ npx agentos start', color: 'text-cream', delay: 0, typing: true },
	{ text: '', delay: 600 },
	{ text: AGENTOS_ASCII, color: 'text-cream', delay: 800 },
	{ text: '', delay: 1200 },
	{ text: '  v0.1.0  |  runtime ready', color: 'text-cream/45', delay: 1400 },
	{ text: '', delay: 1600 },
	{ text: '✓ V8 isolate pool initialized (12 workers)', color: 'text-sage', delay: 1800 },
	{ text: '✓ File system mounted → /workspace', color: 'text-sage', delay: 2100 },
	{ text: '✓ Tool registry loaded (git, curl, python, node)', color: 'text-sage', delay: 2400 },
	{ text: '✓ Network policy applied → allowlist mode', color: 'text-sage', delay: 2700 },
	{ text: '', delay: 3000 },
	{ text: '● Agent session created  sid=a8f3c2e1', color: 'text-cream/85', delay: 3200 },
	{ text: '  → Claude Code connected', color: 'text-cream/55', delay: 3500 },
	{ text: '  → Prompt: "Set up a Next.js app with auth"', color: 'text-cream/55', delay: 3800 },
	{ text: '', delay: 4100 },
	{ text: '  ▸ agent  npm create next-app@latest /workspace/app', color: 'text-cream/70', delay: 4400 },
	{ text: '  ▸ agent  npm install next-auth@5 prisma @prisma/client', color: 'text-cream/70', delay: 5000 },
	{ text: '  ▸ agent  Writing 7 files...', color: 'text-cream/70', delay: 5600 },
	{ text: '  ▸ agent  npx prisma db push', color: 'text-cream/70', delay: 6200 },
	{ text: '', delay: 6800 },
	{ text: '✓ Task complete  duration=14.2s  tokens=3,847  cost=$0.012', color: 'text-sage', delay: 7000 },
	{ text: '  → Preview: http://localhost:3000', color: 'text-cream/55', delay: 7300 },
	{ text: '', delay: 7600 },
	{ text: '● Listening for new sessions...', color: 'text-cream/85', delay: 7800 },
];

const FakeTerminal = () => {
	const [visibleCount, setVisibleCount] = useState(0);
	const [typedText, setTypedText] = useState('');
	const [isTyping, setIsTyping] = useState(false);
	const scrollRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (visibleCount >= terminalLines.length) return;

		const line = terminalLines[visibleCount];
		const prevDelay = visibleCount > 0 ? terminalLines[visibleCount - 1].delay : 0;
		const wait = line.delay - prevDelay;

		const timer = setTimeout(() => {
			if (line.typing) {
				setIsTyping(true);
				setTypedText('');
				let charIdx = 0;
				const typeInterval = setInterval(() => {
					charIdx++;
					setTypedText(line.text.slice(0, charIdx));
					if (charIdx >= line.text.length) {
						clearInterval(typeInterval);
						setIsTyping(false);
						setVisibleCount((c) => c + 1);
					}
				}, 40);
				return () => clearInterval(typeInterval);
			} else {
				setVisibleCount((c) => c + 1);
			}
		}, wait);

		return () => clearTimeout(timer);
	}, [visibleCount]);

	useEffect(() => {
		if (scrollRef.current) {
			scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
		}
	}, [visibleCount, typedText]);

	return (
		<InkPanel>
			<div className='flex items-center gap-2 border-b border-cream/10 px-4 py-3'>
				<div className='h-3 w-3 rounded-full bg-cream/15' />
				<div className='h-3 w-3 rounded-full bg-cream/15' />
				<div className='h-3 w-3 rounded-full bg-cream/15' />
				<span className='ml-2 font-mono text-xs text-cream/45'>terminal</span>
			</div>
			<div
				ref={scrollRef}
				className='h-[360px] overflow-y-auto p-4 font-mono text-[11px] leading-relaxed md:h-[420px] md:text-xs'
			>
				{terminalLines.slice(0, visibleCount).map((line, i) => (
					<div key={i} className={`${line.color || 'text-cream/45'} whitespace-pre-wrap`}>
						{line.text || '\u00A0'}
					</div>
				))}
				{isTyping && (
					<div className={`${terminalLines[visibleCount]?.color || 'text-cream/45'} whitespace-pre-wrap`}>
						{typedText}
						<span className='animate-pulse'>▌</span>
					</div>
				)}
				{visibleCount >= terminalLines.length && (
					<div className='text-cream/45'>
						<span className='animate-pulse'>▌</span>
					</div>
				)}
			</div>
		</InkPanel>
	);
};

// --- Hero Tabs (scrollable with fade + arrows) ---
interface HeroTabEntry {
	key: string;
	icon?: typeof Bot;
	faIcon?: IconProp;
	label: string;
	docsHref: string;
	fileName?: string;
	code?: string;
	highlightedCode?: string;
}

const HeroTabs = ({ tabs, activeTab, onTabChange }: { tabs: HeroTabEntry[]; activeTab: number; onTabChange: (idx: number) => void }) => {
	const scrollRef = useRef<HTMLDivElement>(null);
	const [canScrollLeft, setCanScrollLeft] = useState(false);
	const [canScrollRight, setCanScrollRight] = useState(false);

	const checkOverflow = useCallback(() => {
		const el = scrollRef.current;
		if (!el) return;
		setCanScrollLeft(el.scrollLeft > 2);
		setCanScrollRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 2);
	}, []);

	useEffect(() => {
		const el = scrollRef.current;
		if (!el) return;
		checkOverflow();
		el.addEventListener('scroll', checkOverflow, { passive: true });
		const ro = new ResizeObserver(checkOverflow);
		ro.observe(el);
		return () => {
			el.removeEventListener('scroll', checkOverflow);
			ro.disconnect();
		};
	}, [checkOverflow]);

	const scroll = (direction: 'left' | 'right') => {
		const el = scrollRef.current;
		if (!el) return;
		const amount = el.clientWidth * 0.5;
		el.scrollBy({ left: direction === 'left' ? -amount : amount, behavior: 'smooth' });
	};

	return (
		<div className='relative mb-4 overflow-hidden'>
			{/* Left fade + arrow */}
			{canScrollLeft && (
				<div
					className='absolute inset-y-0 left-0 z-20 flex w-20 items-center justify-start bg-gradient-to-r from-paper from-30% via-paper/70 via-60% to-transparent'
				>
					<button
						type='button'
						onClick={() => scroll('left')}
						className='ml-1 flex h-8 w-8 items-center justify-center rounded-full text-ink-faint hover:text-ink'
						aria-label='Scroll tabs left'
					>
						<ChevronLeft className='h-4 w-4' />
					</button>
				</div>
			)}

			{/* Scrollable tabs */}
			<div ref={scrollRef} className='scrollbar-hide overflow-x-auto border-b border-ink/10'>
				<div className='flex min-w-max flex-nowrap items-center justify-start gap-1'>
					{tabs.map((tab, idx) => {
						const LucideIcon = tab.icon;
						return (
							<button
								key={tab.label}
								type='button'
								onClick={() => onTabChange(idx)}
								className={`-mb-px inline-flex shrink-0 items-center gap-2 whitespace-nowrap border-b-2 px-3 py-2 font-mono text-xs transition-colors md:px-4 ${
									activeTab === idx
										? 'border-pine text-ink'
										: 'border-transparent text-ink-soft hover:text-ink'
								}`}
							>
								{tab.faIcon ? (
									<Icon icon={tab.faIcon} className='h-4 w-4' />
								) : LucideIcon ? (
									<LucideIcon className='h-4 w-4' />
								) : null}
								{tab.label}
							</button>
						);
					})}
				</div>
			</div>

			{/* Right fade + arrow */}
			{canScrollRight && (
				<div
					className='absolute inset-y-0 right-0 z-20 flex w-20 items-center justify-end bg-gradient-to-l from-paper from-30% via-paper/70 via-60% to-transparent'
				>
					<button
						type='button'
						onClick={() => scroll('right')}
						className='mr-1 flex h-8 w-8 items-center justify-center rounded-full text-ink-faint hover:text-ink'
						aria-label='Scroll tabs right'
					>
						<ChevronRight className='h-4 w-4' />
					</button>
				</div>
			)}
		</div>
	);
};

// --- Hero ---
const agents = [
	{ src: '/images/agent-logos/pi.svg', name: 'Pi', comingSoon: false },
	{ src: '/images/agent-logos/claude-code.svg', name: 'Claude Code', comingSoon: true },
	{ src: '/images/agent-logos/codex.svg', name: 'Codex', comingSoon: true },
	{ src: '/images/agent-logos/opencode.svg', name: 'OpenCode', comingSoon: true },
	{ src: '/images/agent-logos/amp.svg', name: 'Amp', comingSoon: true },
];
const heroTabMeta: Array<{ key: string; icon?: typeof Bot; faIcon?: IconProp; label: string; docsHref: string }> = [
	{ key: 'agents', icon: Bot, label: 'Agents', docsHref: '/docs/agent-os/sessions' },
	{ key: 'tools', icon: Wrench, label: 'Tools', docsHref: '/docs/agent-os/tools' },
	{ key: 's3-filesystem', icon: HardDrive, label: 'S3 File System', docsHref: '/docs/agent-os/filesystem' },
	{ key: 'cron', icon: CalendarClock, label: 'Cron', docsHref: '/docs/agent-os/cron' },
	{ key: 'agent-agent', icon: Layers, label: 'Agent-Agent', docsHref: '/docs/agent-os/agent-to-agent' },
	{ key: 'workflows', icon: Workflow, label: 'Workflows', docsHref: '/docs/agent-os/workflows' },
	{ key: 'nodejs', faIcon: faNodeJs, label: 'Node.js', docsHref: '/docs/agent-os/processes' },
	{ key: 'python', faIcon: faPython, label: 'Python', docsHref: '/docs/agent-os/processes' },
	{ key: 'bash', icon: Terminal, label: 'Bash', docsHref: '/docs/agent-os/processes' },
	{ key: 'multiplayer', icon: Users, label: 'Multiplayer', docsHref: '/docs/agent-os/multiplayer' },
	{ key: 'webhooks', icon: Webhook, label: 'Webhooks', docsHref: '/docs/agent-os/webhooks' },
	{ key: 'git', icon: GitBranch, label: 'Git', docsHref: '/docs/agent-os/processes' },
	{ key: 'sandbox', icon: Container, label: 'Sandbox Mounting', docsHref: '/docs/agent-os/sandbox' },
	{ key: 'permissions', icon: ShieldCheck, label: 'Permissions', docsHref: '/docs/agent-os/permissions' },
];

const Hero = ({ heroTabs }: { heroTabs: HeroTabCode[] }) => {
	const [activeTab, setActiveTab] = useState(0);
	const [hoveredAgent, setHoveredAgent] = useState<{ src: string; name: string } | null>(null);
	const [autoPlayAgent, setAutoPlayAgent] = useState<{ src: string; name: string } | null>(null);
	const [autoPlayComplete, setAutoPlayComplete] = useState(false);

	const getStartedTabs = heroTabMeta.map((tab) => ({
		...tab,
		...heroTabs.find((heroTab) => heroTab.key === tab.key),
	}));

	// Auto-cycle through agents starting 2.5s before stroke animation ends
	useEffect(() => {
		const logoAnimationDuration = 800; // Start cycling 2.5s before the 3.3s animation ends
		const agentDisplayDuration = 400; // Time to show each agent

		const startAutoPlay = setTimeout(() => {
			let currentIndex = 0;

			const cycleAgents = () => {
				if (currentIndex < agents.length) {
					setAutoPlayAgent(agents[currentIndex]);
					currentIndex++;
					setTimeout(cycleAgents, agentDisplayDuration);
				} else {
					// End on OS (null)
					setAutoPlayAgent(null);
					setAutoPlayComplete(true);
				}
			};

			cycleAgents();
		}, logoAnimationDuration);

		return () => clearTimeout(startAutoPlay);
	}, []);

	// Displayed agent is either hovered (if autoplay complete) or autoplay agent
	const displayedAgent = autoPlayComplete ? hoveredAgent : autoPlayAgent;

	return (
		<section className='relative flex min-h-[100svh] flex-col justify-center px-6 pt-24 md:pt-24'>
			<div className='mx-auto w-full max-w-5xl'>
				{/* Title */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					animate={{ opacity: 1, y: 0 }}
					transition={{ duration: 0.5, delay: 0.05 }}
					className='mb-6 flex items-center justify-center md:justify-start'
				>
					<div className='relative'>
						<AnimatedAgentOSLogo className='h-12 w-auto md:h-16 lg:h-20' displayedAgent={displayedAgent} />
						<span className='absolute -right-[8px] -top-[7px] rounded-full border border-ink bg-paper px-2 py-0.5 text-[10px] font-medium text-ink'>Beta</span>
					</div>
				</motion.div>

				{/* Subtitle */}
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					animate={{ opacity: 1, y: 0 }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className='mb-10 max-w-2xl text-center text-base text-ink-soft md:text-left md:text-lg'
				>
					A portable open-source operating system for agents. ~6 ms coldstarts, 32x cheaper than sandboxes. Powered by WebAssembly and V8 isolates.
				</motion.p>

				{/* Supported Harnesses */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					animate={{ opacity: 1, y: 0 }}
					transition={{ duration: 0.5, delay: 0.12 }}
					className='mb-10 flex flex-wrap items-center justify-center gap-2 md:justify-start md:gap-4'
				>
					<span className='font-mono text-[11px] uppercase tracking-[0.16em] text-ink-faint'>Works with</span>
					<div className='flex flex-wrap items-center justify-center gap-2 md:justify-start md:gap-4'>
						{agents.map((agent) => (
							<div
								key={agent.name}
								className='flex cursor-pointer items-center gap-1.5 rounded-md px-2 py-1 transition-colors hover:bg-ink/5'
								onMouseEnter={() => autoPlayComplete && setHoveredAgent(agent)}
								onMouseLeave={() => autoPlayComplete && setHoveredAgent(null)}
							>
								<img src={agent.src} alt={agent.name} className='h-4 w-4' />
								<span className='text-sm text-ink-soft'>{agent.name}{agent.comingSoon && '*'}</span>
							</div>
						))}
					</div>
					<span className='text-xs text-ink-faint'>*Coming Soon</span>
				</motion.div>

				{/* Code snippets */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					animate={{ opacity: 1, y: 0 }}
					transition={{ duration: 0.5, delay: 0.15 }}
				>
					{/* Tabs */}
					<HeroTabs tabs={getStartedTabs} activeTab={activeTab} onTabChange={setActiveTab} />

					{/* Code block */}
					<InkPanel caption={<>Fig. 01 — {getStartedTabs[activeTab]?.label ?? 'agentOS'}</>}>
						<div className='flex items-center gap-2 border-b border-cream/10 px-4 py-3'>
							<div className='h-3 w-3 rounded-full bg-cream/15' />
							<div className='h-3 w-3 rounded-full bg-cream/15' />
							<div className='h-3 w-3 rounded-full bg-cream/15' />
							<span className='ml-2 font-mono text-xs text-cream/45'>{getStartedTabs[activeTab]?.fileName ?? 'index.ts'}</span>
						</div>
						<div className='relative h-[380px] overflow-y-auto'>
							<AnimatePresence mode='wait'>
								<motion.div
									key={activeTab}
									initial={{ opacity: 0 }}
									animate={{ opacity: 1 }}
									exit={{ opacity: 0 }}
									transition={{ duration: 0.2 }}
									className='overflow-x-auto p-6 font-mono text-sm leading-relaxed text-cream/80 [&_.line]:break-all [&_.shiki]:!m-0 [&_.shiki]:!bg-transparent [&_.shiki]:!p-0 [&_.shiki]:font-mono [&_.shiki]:text-sm [&_.shiki]:leading-relaxed'
								>
									<span
										className='not-prose code'
										// biome-ignore lint/security/noDangerouslySetInnerHtml: generated from shiki during Astro render
										dangerouslySetInnerHTML={{ __html: getStartedTabs[activeTab]?.highlightedCode ?? '' }}
									/>
								</motion.div>
							</AnimatePresence>
						</div>
					</InkPanel>
				</motion.div>

				{/* Buttons */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					animate={{ opacity: 1, y: 0 }}
					transition={{ duration: 0.5, delay: 0.2 }}
					className='mt-6 flex flex-col items-center gap-3 sm:flex-row sm:items-center md:items-start w-full'
				>
					<a
						href='/docs/agent-os'
						className='selection-dark inline-flex w-full items-center justify-center gap-2 whitespace-nowrap rounded-md bg-accent-deep px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent sm:w-auto'
					>
						Read the Docs
						<ArrowRight className='h-4 w-4' />
					</a>
					<CopyCommand command='npm install rivetkit' />
					<div className='flex-1' />
					<a
						href='/agent-os/registry'
						className='inline-flex items-center gap-2 whitespace-nowrap text-sm text-ink-soft transition-colors hover:text-pine'
					>
						<Package className='h-4 w-4' />
						View Package Registry
						<ArrowRight className='h-4 w-4' />
					</a>
				</motion.div>

			</div>
		</section>
	);
};


// --- Feature Card ---
const FeatureCard = ({
	icon: IconComponent,
	title,
	description,
	tags,
	metric,
	delay = 0,
}: {
	icon: React.ComponentType<{ className?: string }>;
	title: string;
	description: string;
	tags?: string[];
	metric?: { value: string; label: string };
	delay?: number;
}) => (
	<motion.div
		initial={{ opacity: 0, y: 20 }}
		whileInView={{ opacity: 1, y: 0 }}
		viewport={{ once: true }}
		transition={{ duration: 0.5, delay }}
		className='border-t border-ink/10 pt-6'
	>
		<div className='mb-3 text-olive'>
			<IconComponent className='h-4 w-4' />
		</div>
		<h3 className='mb-2 text-base font-medium text-ink'>
			{title}
		</h3>
		<p className='mb-4 text-sm leading-relaxed text-ink-soft'>{description}</p>
		{tags && (
			<div className='flex flex-wrap gap-2'>
				{tags.map((tag) => (
					<span
						key={tag}
						className='rounded bg-ink/5 px-2.5 py-1 font-mono text-xs text-ink-soft'
					>
						{tag}
					</span>
				))}
			</div>
		)}
		{metric && (
			<div className='flex items-baseline gap-2'>
				<span className='font-mono text-3xl font-medium text-ink'>
					{metric.value}
				</span>
				<span className='text-sm text-ink-soft'>{metric.label}</span>
			</div>
		)}
	</motion.div>
);

const DocsLink = ({ href }: { href: string }) => (
	<a
		href={href}
		className='inline-flex items-center gap-1 text-sm text-ink-soft transition-colors hover:text-pine'
	>
		Docs <span aria-hidden='true'>→</span>
	</a>
);

// --- Icon Box (rounded square outline like Rivet/agentOS logos) ---
const IconBox = ({ children }: { children: React.ReactNode }) => (
	<div className='relative mb-6 flex h-10 w-10 items-center justify-center text-olive md:h-12 md:w-12'>
		<svg
			className='absolute inset-0 h-full w-full'
			viewBox='0 0 172 172'
			fill='none'
		>
			<rect
				x='8'
				y='8'
				width='156'
				height='156'
				rx='40'
				ry='40'
				stroke='currentColor'
				strokeWidth='10'
				fill='none'
			/>
		</svg>
		{children}
	</div>
);

// --- Sticky Stacking Feature Card ---
interface StackFeature {
	icon: React.ComponentType<{ className?: string }>;
	title: string;
	description: string;
	detail?: string;
	tags?: string[];
	metric?: { value: string; label: string };
}

// --- Themed Feature Sections (card carousel) ---
interface ThemedFeature {
	title: string;
	description: string;
	icon: React.ComponentType<{ className?: string }>;
	comingSoon?: boolean;
	docsHref?: string;
}

interface ThemedSection {
	category: string;
	title: string;
	subtitle: string;
	features: ThemedFeature[];
}

const themedSections: ThemedSection[] = [
	{
		category: 'Agents',
		title: 'Agents that just work.',
		subtitle: 'Every agent deserves a runtime that understands it.',
		features: [
			{ icon: Bot, title: 'Supports Claude Code, Codex, OpenCode, Amp, and more', description: 'Run any coding agent with a single unified API. Swap agents without changing your infrastructure.' },
			{ icon: Code, title: 'Simple sessions API', description: 'Create, manage, and resume agent sessions with a few lines of code. State persists automatically.', docsHref: '/docs/agent-os/sessions' },
			{ icon: Activity, title: 'Embedded LLM metering', description: 'Track token usage, cost, and latency per agent. No per-agent API keys needed. The host handles credential scoping.', comingSoon: true, docsHref: '/docs/agent-os/llm-gateway' },
			{ icon: Layers, title: 'Universal transcript format', description: 'One transcript format across all agents. Powered by ACP. Compare, debug, and audit any session.', docsHref: '/docs/agent-os/sessions' },
			{ icon: Clock, title: 'Automatic transcript persistence', description: 'Every conversation is saved. Replay sessions, audit behavior, and build on past context without extra code.', docsHref: '/docs/agent-os/persistence' },
		],
	},
	{
		category: 'Infrastructure',
		title: 'Infrastructure that disappears.',
		subtitle: 'Deploy anywhere. Scale to anything. Forget about servers.',
		features: [
			{ icon: Globe, title: 'Runs on Rivet Cloud or your infra', description: 'Managed hosting or self-hosted. Same API, same experience, your choice of where it runs.', docsHref: '/docs/agent-os/deployment' },
			{ icon: Terminal, title: 'Easy to deploy on prem', description: 'A single npm package. No Kubernetes operators, no sidecar containers. Just install and run.', docsHref: '/docs/agent-os/deployment' },
			{ icon: Clock, title: 'Low overhead', description: 'No VMs to boot. No containers to pull. Start in milliseconds with minimal memory footprint.' },
			{ icon: FolderOpen, title: 'Mount anything as a file system', description: 'S3, GitHub, databases. No per-agent credentials needed. The host handles access scoping.', docsHref: '/docs/agent-os/filesystem' },
			{ icon: Shield, title: 'Extend with a sandbox when needed', description: 'agentOS handles most tasks, but pairs seamlessly with sandboxes for heavier workloads.', docsHref: '/docs/agent-os/sandbox' },
		],
	},
	{
		category: 'Orchestration',
		title: 'Orchestration without complexity.',
		subtitle: 'Coordinate agents, humans, and systems out of the box.',
		features: [
			{ icon: Shield, title: 'Authentication', description: 'Authenticate agent connections with your existing auth model. Validate credentials and attach user state on connect.', docsHref: '/docs/agent-os/authentication' },
			{ icon: Globe, title: 'Webhooks', description: 'Receive external events and route them into agents with lightweight HTTP handlers and durable queues.', docsHref: '/docs/agent-os/webhooks' },
			{ icon: Bot, title: 'Multiplayer & Realtime', description: 'Multiple clients can observe and collaborate with the same agent environment in real time.', docsHref: '/docs/agent-os/multiplayer' },
			{ icon: Layers, title: 'Agent-to-Agent', description: 'Let agents delegate work to other agents through host-defined tools and shared orchestration flows.', docsHref: '/docs/agent-os/agent-to-agent' },
			{ icon: Wrench, title: 'Workflows', description: 'Chain agent tasks into durable workflows with retries, branching, and resumable execution built in.', docsHref: '/docs/agent-os/workflows' },
			{ icon: HardDrive, title: 'Queues', description: 'Serialize agent work with durable queues for backpressure, async processing, and ordered execution.', docsHref: '/docs/agent-os/queues' },
			{ icon: Code, title: 'SQLite', description: 'Give agents access to a persistent SQLite database through host tools for structured state and queryable memory.', docsHref: '/docs/agent-os/sqlite' },
		],
	},
	{
		category: 'Security',
		title: 'Security without compromise.',
		subtitle: 'The same isolation technology trusted by browsers worldwide.',
		features: [
			{ icon: Activity, title: 'Restrict CPU and memory granularly', description: 'Set precise resource limits per agent. No runaway processes, no noisy neighbors.', docsHref: '/docs/agent-os/security' },
			{ icon: Globe, title: 'Programmatic network control', description: 'Allow, deny, or proxy any outbound connection. Full control over what your agents can reach.', docsHref: '/docs/agent-os/security' },
			{ icon: Shield, title: 'Custom authentication', description: 'Bring your own auth. API keys, OAuth, JWTs. Agents authenticate on your terms.', docsHref: '/docs/agent-os/authentication' },
			{ icon: Layers, title: 'Isolated private network', description: 'Each agent runs in its own network namespace. No cross-talk between tenants.', docsHref: '/docs/agent-os/security' },
			{ icon: HardDrive, title: 'Powered by WebAssembly and V8 isolates', description: 'The same sandboxing technology behind Google Chrome. Battle-tested at planet scale.', docsHref: '/docs/agent-os/architecture' },
		],
	},
];

const StackingFeatureCards = () => {
	const CARD_HEIGHT = 560;
	const STACK_OFFSET = 12;
	const sectionRef = useRef<HTMLElement>(null);
	const [isInView, setIsInView] = useState(false);

	useEffect(() => {
		const section = sectionRef.current;
		if (!section) return;

		const observer = new IntersectionObserver(
			([entry]) => {
				setIsInView(entry.isIntersecting);
			},
			{ threshold: 0.1 }
		);

		observer.observe(section);
		return () => observer.disconnect();
	}, []);

	const coldStartP99 = benchColdStart[2]; // p99
	const awsArmAgentCost = benchWorkloads.agent.cost[0]; // AWS ARM

	const stackFeatures = [
		// { icon: Clock, title: 'Low overhead and cost.', description: 'No VMs to boot. No containers to pull. Start in milliseconds with minimal memory footprint.', detail: 'Traditional sandboxes take seconds to spin up and consume hundreds of megabytes. agentOS starts instantly and runs lean, so you can scale to thousands of agents without the cost. More details in benchmarks below.', metrics: [{ value: `~${Math.round(coldStartP99.agentOS)}ms`, label: 'p99 coldstart' }, { value: `${awsArmAgentCost.ratio}x`, label: 'cheaper than sandboxes' }] },
		{ icon: Terminal, title: 'Embed in your backend.', detail: 'Your APIs. Your toolchains. No complex agent authentication needed. Just JavaScript functions or hooks.' },
		{ icon: FolderOpen, title: 'Mount anything as a file system.', description: 'S3, SQLite, Google Drive, or the host file system. No per-agent credentials needed.', detail: 'Agents think in files. agentOS lets you expose any storage backend as a familiar directory tree. The host handles credential scoping, so agents never see API keys or secrets.' },
		{ icon: Shield, title: 'Granular security.', detail: 'Fully configurable network and file system security. Control rate limits, bandwidth limits, and file system permissions. Set precise CPU and memory limitations per agent.' },
		{ icon: Globe, title: 'Your laptop, your infra, or on-prem.', description: 'Railway, Vercel, Kubernetes, and more. Deploy wherever your code already runs.', detail: 'agentOS is just an npm package. No vendor lock-in, no special infrastructure. Your agents run in your stack, on your terms.', tags: ['Rivet', 'Railway', 'Vercel', 'Kubernetes', 'ECS', 'Lambda', 'Google Cloud Run'] },
	];

	return (
		<section ref={sectionRef} className='border-t border-ink/10'>
			{/* Fade gradient overlay at bottom - only show when section is in view */}
			{isInView && (
				<div
					className='pointer-events-none fixed bottom-0 left-0 right-0 z-20 h-64'
					style={{
						background: 'linear-gradient(to top, #EFEFEF 0%, #EFEFEF 20%, transparent 100%)',
					}}
				/>
			)}
			<div
				className='sticky z-0 px-6 pb-12 pt-24 md:pt-32'
				style={{ top: '60px' }}
			>
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className='mx-auto max-w-4xl text-center text-3xl font-medium tracking-[-0.015em] text-ink md:text-5xl'
				>
					Meet your agent&apos;s new operating system.
				</motion.h2>
			</div>
			<div
				className='relative'
				style={{ height: `${stackFeatures.length * CARD_HEIGHT + 500}px` }}
			>
				<div className='sticky top-0 px-6 pt-8'>
					<div className='mx-auto max-w-4xl relative'>
						{stackFeatures.map((feature, idx) => {
							const Icon = feature.icon;
							return (
								<div
									key={feature.title}
									className='sticky'
									style={{
										top: `${280 + idx * STACK_OFFSET}px`,
										zIndex: idx + 1,
									}}
								>
									<div
										className='mb-6 flex min-h-0 flex-col rounded-2xl border border-ink/15 bg-paper-mid p-8 md:p-12'
										style={{
											minHeight: `${CARD_HEIGHT - 24}px`,
										}}
									>
										<IconBox>
											<Icon className='h-4 w-4 text-olive md:h-5 md:w-5' />
										</IconBox>
										<h2 className='mb-4 text-3xl font-medium tracking-[-0.015em] text-ink md:text-4xl'>
											{feature.title}
										</h2>
										<p className='mb-4 max-w-2xl text-base leading-relaxed text-ink-soft md:text-lg'>
											{feature.description}
										</p>
										{feature.detail && (
											<p className='mb-6 max-w-2xl text-sm leading-relaxed text-ink-soft md:text-base'>
												{feature.detail}
											</p>
										)}
										{feature.tags && (
											<div className='mb-4 flex flex-wrap gap-2'>
												{feature.tags.map((tag) => (
													<span
														key={tag}
														className='rounded-full border border-ink/10 bg-white/55 px-4 py-1.5 font-mono text-sm text-ink-soft'
													>
														{tag}
													</span>
												))}
											</div>
										)}
										{feature.metrics && (
											<div className='grid grid-cols-2 gap-8 md:gap-12'>
												{feature.metrics.map((m) => (
													<div key={m.value} className='flex flex-col'>
														<span className='font-mono text-5xl font-medium text-ink md:text-7xl'>
															{m.value}
														</span>
														<span className='mt-2 text-sm text-ink-soft md:text-base'>{m.label}</span>
													</div>
												))}
											</div>
											)}
									</div>
								</div>
							);
						})}
					</div>
				</div>
			</div>
		</section>
	);
};

const FeatureCardCarousel = ({ section }: { section: ThemedSection }) => {
	const scrollRef = useRef<HTMLDivElement>(null);
	const [canScrollLeft, setCanScrollLeft] = useState(false);
	const [canScrollRight, setCanScrollRight] = useState(true);

	const checkScroll = useCallback(() => {
		const el = scrollRef.current;
		if (!el) return;
		setCanScrollLeft(el.scrollLeft > 4);
		setCanScrollRight(el.scrollLeft < el.scrollWidth - el.clientWidth - 4);
	}, []);

	useEffect(() => {
		const el = scrollRef.current;
		if (!el) return;
		checkScroll();
		el.addEventListener('scroll', checkScroll, { passive: true });
		window.addEventListener('resize', checkScroll);
		return () => {
			el.removeEventListener('scroll', checkScroll);
			window.removeEventListener('resize', checkScroll);
		};
	}, [checkScroll]);

	const scroll = (dir: 'left' | 'right') => {
		const el = scrollRef.current;
		if (!el) return;
		const cardWidth = el.querySelector('div')?.offsetWidth ?? 300;
		el.scrollBy({ left: dir === 'left' ? -cardWidth - 16 : cardWidth + 16, behavior: 'smooth' });
	};

	return (
		<div>
			{/* Cards */}
			<div
				ref={scrollRef}
				className='-mx-6 flex gap-4 overflow-x-auto px-6 pb-4 scrollbar-hide'
				style={{ scrollbarWidth: 'none', msOverflowStyle: 'none' }}
			>
				{section.features.map((feature) => {
					const Icon = feature.icon;
						return (
							<div
								key={feature.title}
								className='relative flex w-[280px] flex-shrink-0 flex-col rounded-2xl border border-ink/10 bg-white/55 p-6'
							>
							{feature.comingSoon && (
								<span className='absolute top-4 right-4 rounded-full border border-ink/10 bg-mat px-2 py-0.5 text-[10px] font-medium text-ink-soft'>
									Coming Soon
								</span>
							)}
							<div className='mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-ink/5'>
								<Icon className='h-5 w-5 text-olive' />
							</div>
							<h3 className='mb-2 text-sm font-medium text-ink'>
								{feature.title}
							</h3>
								<p className='text-sm leading-relaxed text-ink-soft'>
									{feature.description}
								</p>
								{feature.docsHref && (
									<div className='mt-auto pt-4'>
										<DocsLink href={feature.docsHref} />
									</div>
								)}
							</div>
					);
				})}
			</div>

			{/* Navigation */}
			<div className='mt-4 flex items-center justify-end gap-2'>
				<button
					onClick={() => scroll('left')}
					disabled={!canScrollLeft}
					className={`flex h-9 w-9 items-center justify-center rounded-xl border transition-colors ${
						canScrollLeft
							? 'border-ink/20 text-ink hover:border-ink/40'
							: 'border-ink/10 text-ink/25 cursor-default'
					}`}
				>
					<ChevronLeft className='h-4 w-4' />
				</button>
				<button
					onClick={() => scroll('right')}
					disabled={!canScrollRight}
					className={`flex h-9 w-9 items-center justify-center rounded-xl border transition-colors ${
						canScrollRight
							? 'border-ink/20 text-ink hover:border-ink/40'
							: 'border-ink/10 text-ink/25 cursor-default'
					}`}
				>
					<ChevronRight className='h-4 w-4' />
				</button>
			</div>
		</div>
	);
};

const ThemedFeatureSections = () => (
	<div className='mt-16 md:mt-48'>
		{themedSections.map((section, sectionIdx) => (
			<section
				key={section.category}
				className='border-t border-ink/10 px-6 py-24 md:py-40'
			>
				<div className='mx-auto max-w-7xl'>
					{/* Section header */}
					<motion.div
						initial={{ opacity: 0, y: 30 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.6 }}
						className='mb-10'
					>
						<Eyebrow
							index={String(sectionIdx + 1).padStart(2, '0')}
							label={section.category}
							className='mb-4'
						/>
						<h2 className='mb-4 text-3xl font-medium tracking-[-0.015em] text-ink md:text-5xl lg:text-6xl'>
							{section.title}
						</h2>
						<p className='max-w-xl text-base text-ink-soft md:text-lg'>
							{section.subtitle}
						</p>
					</motion.div>

					{/* Card carousel */}
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
					>
						<FeatureCardCarousel section={section} />
					</motion.div>
				</div>
			</section>
		))}
	</div>
);

// --- agentOS Features Section ---
const RegistryCallout = () => (
	<section className='border-t border-ink/10 px-6 py-24 md:py-40'>
		<div className='mx-auto max-w-7xl'>
			<motion.div
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className='rounded-xl border border-ink/10 bg-white/55 p-8 md:p-12'
			>
				<div className='flex flex-col items-start gap-6 md:flex-row md:items-center md:justify-between'>
					<div>
						<h3 className='mb-2 text-2xl font-medium tracking-[-0.015em] text-ink md:text-3xl'>
							agentOS Registry
						</h3>
						<p className='max-w-lg text-base leading-relaxed text-ink-soft'>
							Browse and install pre-built tools, integrations, and capabilities for your agents. From file systems to databases to API connectors.
						</p>
					</div>
					<a
						href='/agent-os/registry'
						className='selection-dark inline-flex flex-shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-md bg-ink px-4 py-2 text-sm font-medium text-cream transition-colors hover:bg-ink/85'
					>
						Explore the Registry
						<ArrowRight className='h-4 w-4' />
					</a>
				</div>
			</motion.div>
		</div>
	</section>
);

const AgentOSFeatures = () => (
	<div id='agentos'>
		<StackingFeatureCards />
		<ThemedFeatureSections />
		<RegistryCallout />
	</div>
);

// --- Benchmarks ---
// Benchmark data (computed from raw inputs in bench.ts)
import { benchColdStart, benchWorkloads, BENCHMARK_DATE, SANDBOX_COLDSTART_PROVIDER, SANDBOX_COST_PROVIDER, type WorkloadKey } from '@/data/bench';

// Floor for pixel-grid columns so the tiny agentOS values still render a visible cell.
const BENCH_CHART_FLOOR = 0.09;

function BenchInfoTooltip({ children }: { children: React.ReactNode }) {
	// The wrapper is intentionally not positioned so the tooltip spans the ink
	// card itself (the nearest positioned ancestor) instead of clipping at the
	// panel's overflow-hidden edge.
	return (
		<span className='group/tip ml-1.5 inline-flex align-middle'>
			<svg
				className='h-3.5 w-3.5 cursor-help text-cream/35 transition-colors group-hover/tip:text-cream/70'
				viewBox='0 0 16 16'
				fill='currentColor'
			>
				<path d='M8 0a8 8 0 100 16A8 8 0 008 0zm1 12H7V7h2v5zm-1-6a1 1 0 110-2 1 1 0 010 2z' />
			</svg>
			<span className='pointer-events-none absolute inset-x-3 bottom-12 z-50 rounded-lg border border-cream/15 bg-ink p-3 text-left text-[11px] leading-relaxed text-cream/80 opacity-0 shadow-xl transition-opacity duration-200 group-hover/tip:pointer-events-auto group-hover/tip:opacity-100 [&_a]:text-sage [&_a]:underline [&_a]:underline-offset-2 [&_strong]:font-medium [&_strong]:text-cream'>
				{children}
			</span>
		</span>
	);
}

function BenchToggle({ options, active, onChange }: { options: string[]; active: number; onChange: (idx: number) => void }) {
	return (
		<div className='flex flex-wrap justify-end gap-1'>
			{options.map((label, i) => (
				<button
					key={label}
					onClick={() => onChange(i)}
					className={`rounded px-2 py-1 font-mono text-[10px] tracking-wider transition-colors ${
						i === active ? 'bg-cream/10 text-cream' : 'text-cream/45 hover:text-cream/70'
					}`}
				>
					{label}
				</button>
			))}
		</div>
	);
}

interface BenchRowEntry {
	label: React.ReactNode;
	value: string;
	highlight?: boolean;
}

// Dark ink data card matching the BenchmarksSection pattern on /actors:
// sage mono title, direction tag, headline stat beside a pixel-grid chart,
// and label/value rows pinned to the card's foot.
function BenchCard({
	title,
	stat,
	statNote,
	chart,
	toggle,
	rows,
	note,
}: {
	title: string;
	stat: string;
	statNote: string;
	chart: number[];
	toggle?: React.ReactNode;
	rows: BenchRowEntry[];
	note?: string;
}) {
	return (
		<InkPanel className='h-full'>
			<div className='flex h-full flex-col p-6'>
				<div className='mb-5 flex items-start justify-between gap-4'>
					<span className='font-mono text-[11px] uppercase tracking-[0.16em] text-sage'>{title}</span>
					<span className='flex items-center gap-1 text-right font-mono text-[10px] uppercase tracking-wider text-cream/40'>
						<ArrowDown className='h-3 w-3 flex-shrink-0' />
						lower is better
					</span>
				</div>

				<div className='flex items-end justify-between gap-6'>
					<div className='min-w-0'>
						<div className='text-2xl font-medium leading-tight text-cream md:text-3xl'>{stat}</div>
						<div className='mt-1 text-sm text-cream/55'>{statNote}</div>
					</div>
					<PixelGridChart values={chart} rows={7} accentColumn={0} className='h-20 w-auto flex-shrink-0' />
				</div>

				{toggle ? <div className='mt-5 flex justify-end'>{toggle}</div> : null}

				<div className='mt-6 flex flex-1 flex-col justify-end gap-2 border-t border-cream/10 pt-4'>
					{rows.map((row, i) => (
						<div key={i} className='flex items-center justify-between gap-4 text-xs'>
							<span className={`inline-flex items-center ${row.highlight ? 'text-cream' : 'text-cream/55'}`}>
								{row.label}
							</span>
							<span className={row.highlight ? 'font-medium text-sage' : 'text-cream/55'}>{row.value}</span>
						</div>
					))}
					{note ? <p className='mt-1 text-[11px] leading-relaxed text-cream/40'>{note}</p> : null}
				</div>
			</div>
		</InkPanel>
	);
}

function BenchColdStartChart() {
	const groups = benchColdStart;
	const [active, setActive] = useState(2);
	const g = groups[active];

	return (
		<BenchCard
			title='Cold Start'
			stat={`${g.agentOS} ms`}
			statNote={`${Math.round(g.sandbox / g.agentOS)}x faster`}
			chart={[Math.max(g.agentOS / g.sandbox, BENCH_CHART_FLOOR), 1]}
			toggle={<BenchToggle options={groups.map((t) => t.label)} active={active} onChange={setActive} />}
			rows={[
				{
					label: (
						<>
							agentOS
							<BenchInfoTooltip>
								<strong>What&apos;s measured:</strong> Time from requesting an execution to first code running.
								<br /><br />
								<strong>Why the gap:</strong> agentOS runs agents in-process — V8 isolates and Wasm inside your host. No VM to boot, no network hop, no disk image. Sandboxes must boot an entire environment, allocate memory, and establish a network connection before code can run.
								<br /><br />
								<strong>Sandbox baseline:</strong> {SANDBOX_COLDSTART_PROVIDER}, the fastest mainstream sandbox provider as of {BENCHMARK_DATE}.
								<br /><br />
								<strong>agentOS:</strong> Median of 10,000 runs (100 iterations x 100 samples) on Intel i7-12700KF.
							</BenchInfoTooltip>
						</>
					),
					value: `${g.agentOS} ms`,
					highlight: true,
				},
				{ label: 'Fastest sandbox', value: `${g.sandbox.toLocaleString()} ms` },
			]}
		/>
	);
}

function BenchMemoryBar({ workload }: { workload: WorkloadKey }) {
	const mem = benchWorkloads[workload].memory;

	return (
		<BenchCard
			title='Memory Per Instance'
			stat={mem.agentOS}
			statNote={mem.multiplier}
			chart={[Math.max(mem.agentOSBar / 100, BENCH_CHART_FLOOR), 1]}
			rows={[
				{
					label: (
						<>
							agentOS
							<BenchInfoTooltip>
								<strong>What&apos;s measured:</strong> Memory footprint added per concurrent execution.
								<br /><br />
								<strong>Why the gap:</strong> In-process isolates share the host's memory. Each additional execution only adds its own heap and stack. Sandboxes allocate a dedicated environment with a minimum memory reservation, even if the code inside uses far less.
								<br /><br />
								<strong>Sandbox baseline:</strong> {SANDBOX_COST_PROVIDER}, the cheapest mainstream sandbox provider as of {BENCHMARK_DATE}. Default sandbox: 1 vCPU + 1 GiB RAM.
								<br /><br />
								<strong>agentOS:</strong> {workload === 'agent' ? `${benchWorkloads.agent.memory.agentOS} for a full Pi coding agent session with MCP servers and file system mounts.` : `${benchWorkloads.shell.memory.agentOS} for the minimal shell workload under sustained load.`}
							</BenchInfoTooltip>
						</>
					),
					value: mem.agentOS,
					highlight: true,
				},
				{ label: 'Cheapest sandbox', value: mem.sandbox },
			]}
			note='Sandboxes reserve idle RAM per agent.'
		/>
	);
}

function BenchCostChart({ workload }: { workload: WorkloadKey }) {
	const tiers = benchWorkloads[workload].cost;
	const sandboxCost = benchWorkloads[workload].sandboxCost;
	const [active, setActive] = useState(0);
	const t = tiers[active];

	return (
		<BenchCard
			title='Cost Per Execution-Second'
			stat={t.value}
			statNote={t.multiplier}
			chart={[Math.max(t.bar / 100, BENCH_CHART_FLOOR), 1]}
			toggle={<BenchToggle options={tiers.map((tier) => tier.label)} active={active} onChange={setActive} />}
			rows={[
				{
					label: (
						<>
							agentOS
							<BenchInfoTooltip>
								<strong>What&apos;s measured:</strong> <code className='rounded bg-cream/10 px-1 py-0.5 text-[10px]'>server price per second / concurrent executions per server</code>
								<br /><br />
								<strong>Why it&apos;s cheaper:</strong> Each execution uses {benchWorkloads[workload].memory.agentOS} instead of a {benchWorkloads[workload].memory.sandbox} sandbox minimum. And you run on your own hardware, which is significantly cheaper than per-second sandbox billing.
								<br /><br />
								<strong>Sandbox baseline:</strong> {SANDBOX_COST_PROVIDER}, the cheapest mainstream sandbox provider as of {BENCHMARK_DATE}. Default sandbox: 1 vCPU + 1 GiB RAM at $0.0504/vCPU-h + $0.0162/GiB-h.
								<br /><br />
								<strong>agentOS:</strong> {benchWorkloads[workload].memory.agentOS} baseline per execution, assuming 70% utilization (industry-standard HPA scaling threshold). Select a hardware tier above to compare.
							</BenchInfoTooltip>
						</>
					),
					value: t.value,
					highlight: true,
				},
				{ label: 'Cheapest sandbox', value: sandboxCost },
			]}
			note='Assumes one agent per sandbox, needed for isolation.'
		/>
	);
}

function BenchmarkSection() {
	const [workload, setWorkload] = useState<WorkloadKey>('agent');
	const wl = benchWorkloads[workload];

	return (
		<motion.div
			initial={{ opacity: 0, y: 20 }}
			whileInView={{ opacity: 1, y: 0 }}
			viewport={{ once: true }}
			transition={{ duration: 0.5 }}
		>
			<div className='mb-8'>
				<h3 className='mb-2 text-2xl font-medium tracking-[-0.015em] text-ink md:text-3xl'>
					Performance benchmarks
				</h3>
				<p className='text-base leading-relaxed text-ink-soft'>
					agentOS vs. traditional sandboxes.
				</p>
			</div>

			<div className='mb-6 flex items-center justify-between max-sm:flex-col max-sm:items-stretch max-sm:gap-2'>
				<p className='text-xs text-ink-faint max-sm:order-2 max-sm:px-1 max-sm:leading-relaxed'>Workload: {wl.description}</p>
				<div className='flex gap-1 rounded-lg border border-ink/10 bg-white/55 p-1 max-sm:order-1 max-sm:grid max-sm:w-full max-sm:grid-cols-2 max-sm:rounded-xl'>
					{(Object.keys(benchWorkloads) as WorkloadKey[]).map((key) => (
						<button
							key={key}
							onClick={() => setWorkload(key)}
							aria-pressed={workload === key}
							className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors max-sm:flex max-sm:min-h-10 max-sm:w-full max-sm:items-center max-sm:justify-center max-sm:rounded-lg max-sm:py-2 max-sm:text-center ${
								workload === key ? 'bg-ink text-cream' : 'text-ink-soft hover:text-ink'
							}`}
						>
							{benchWorkloads[key].label}
						</button>
					))}
				</div>
			</div>

			<div className='grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3'>
				<BenchColdStartChart />
				<BenchMemoryBar workload={workload} />
				<BenchCostChart workload={workload} />
			</div>

			<p className='mt-8 font-mono text-xs leading-relaxed text-ink-faint'>
				Measured on Intel i7-12700KF. Cold start baseline: {SANDBOX_COLDSTART_PROVIDER}, the fastest mainstream sandbox provider as of {BENCHMARK_DATE}. Cost baseline: {SANDBOX_COST_PROVIDER}, the cheapest mainstream sandbox provider as of {BENCHMARK_DATE} (1 vCPU + 1 GiB default). Cost assumes 70% utilization on self-hosted hardware vs. per-second sandbox billing.{' '}
				<a
					href='/docs/agent-os/benchmarks'
					className='inline-flex items-center gap-1 text-ink-soft underline underline-offset-2 transition-colors hover:text-pine'
				>
					Benchmark document
					<ExternalLink className='h-3 w-3' />
				</a>
			</p>
		</motion.div>
	);
}

const TechnologyAndBenchmarks = () => (
	<section className='border-t border-ink/10 py-16 md:py-32'>
		<div className='mx-auto max-w-5xl px-6'>
			{/* Technology intro */}
			<motion.div
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className='mb-16'
			>
				<h2 className='mb-4 text-3xl font-medium tracking-[-0.015em] text-ink md:text-5xl'>
					A new operating system architecture.
				</h2>
				<p className='mb-6 max-w-3xl text-base leading-relaxed text-ink-soft md:text-lg'>
					Built from the ground up for lightweight agents. agentOS provides the flexibility of Linux with lower overhead than sandboxes.
				</p>
				<div className='grid gap-6 md:grid-cols-2'>
					<div className='rounded-xl border border-ink/10 bg-white/55 p-6'>
						<div className='mb-3 flex items-center gap-3'>
							<div className='flex h-10 w-10 items-center justify-center rounded-lg bg-ink/5'>
								<img src='/images/agent-os/webassembly-logo.svg' alt='WebAssembly' className='h-6 w-6 grayscale opacity-70' />
							</div>
							<h3 className='text-lg font-medium text-ink'>WebAssembly + V8 Isolates</h3>
						</div>
						<p className='text-sm leading-relaxed text-ink-soft'>
							High-performance virtualization without specialized infrastructure. The same battle-hardened isolation technology that powers Google Chrome.
						</p>
					</div>
					<div className='rounded-xl border border-ink/10 bg-white/55 p-6'>
						<div className='mb-3 flex items-center gap-3'>
							<div className='flex h-10 w-10 items-center justify-center rounded-lg bg-ink/5'>
								<Globe className='h-5 w-5 text-olive' />
							</div>
							<h3 className='text-lg font-medium text-ink'>Battle-tested technology</h3>
						</div>
						<p className='text-sm leading-relaxed text-ink-soft'>
							You&apos;re probably using this technology right now to view this page. Bring the same power to your agents. No VMs, no containers, no overhead.
						</p>
					</div>
				</div>
			</motion.div>

			{/* Benchmarks */}
			<BenchmarkSection />

		</div>
	</section>
);

// --- Before/After Slider ---
const BeforeAfterSlider = ({ before, after }: { before: string; after: string }) => {
	const containerRef = useRef<HTMLDivElement>(null);
	const [position, setPosition] = useState(50);

	const updatePosition = useCallback((clientX: number) => {
		const el = containerRef.current;
		if (!el) return;
		const rect = el.getBoundingClientRect();
		const pct = ((clientX - rect.left) / rect.width) * 100;
		setPosition(Math.max(0, Math.min(100, pct)));
	}, []);

	return (
		<div
			ref={containerRef}
			className='relative select-none overflow-hidden rounded-xl cursor-ew-resize'
			style={{ aspectRatio: '4/3' }}
			onMouseMove={(e) => updatePosition(e.clientX)}
			onTouchMove={(e) => updatePosition(e.touches[0].clientX)}
		>
			{/* After (full) */}
			<img src={after} alt='After' className='absolute inset-0 h-full w-full object-cover' loading='lazy' />
			{/* Before (clipped) */}
			<div className='absolute inset-0 overflow-hidden' style={{ width: `${position}%` }}>
				<img src={before} alt='Before' className='h-full w-full object-cover' style={{ width: `${containerRef.current?.offsetWidth ?? 1000}px`, maxWidth: 'none' }} loading='lazy' />
			</div>
			{/* Divider */}
			<div className='absolute top-0 bottom-0 z-10 w-0.5 bg-white' style={{ left: `${position}%` }}>
				<div className='absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 flex h-8 w-8 items-center justify-center rounded-full bg-white shadow-lg cursor-ew-resize'>
					<svg width='16' height='16' viewBox='0 0 16 16' fill='none'><path d='M5 3L2 8L5 13M11 3L14 8L11 13' stroke='#1B1916' strokeWidth='1.5' strokeLinecap='round' strokeLinejoin='round'/></svg>
				</div>
			</div>
			{/* Labels */}
			<div className='absolute inset-0 overflow-hidden z-10 pointer-events-none' style={{ width: `${position}%` }}>
				<span className='absolute bottom-3 left-3 whitespace-nowrap rounded-full bg-black/50 px-2.5 py-1 text-xs font-medium text-white backdrop-blur-sm'>Unix Operators</span>
			</div>
			<div className='absolute inset-0 overflow-hidden z-10 pointer-events-none' style={{ left: `${position}%`, width: `${100 - position}%` }}>
				<span className='absolute bottom-3 right-3 whitespace-nowrap rounded-full bg-black/50 px-2.5 py-1 text-xs font-medium text-white backdrop-blur-sm'>agentOS Operators</span>
			</div>
		</div>
	);
};

// --- From Unix to Agents ---
const SisterProducts = () => {
	const products = [
		{
			name: 'Secure Exec',
			tagline: 'Secure Node.js execution without a sandbox.',
			bullets: [
				'V8 isolates with bridged Node APIs',
				'npm-compatible: fs, child_process, http',
				'176x faster cold start than containers',
				'Just `npm install` — no Docker, no VMs',
			],
			href: 'https://secureexec.dev/',
			cta: 'secureexec.dev',
		},
		{
			name: 'Sandbox Agent SDK',
			tagline: 'Run coding agents in sandboxes. Control them over HTTP.',
			bullets: [
				'One interface for Claude Code, Codex, OpenCode, Amp',
				'Streams events, handles permissions, manages sessions',
				'Replay, audit, and retain full transcripts',
				'Swap agents with a config change',
			],
			href: 'https://sandboxagent.dev/',
			cta: 'sandboxagent.dev',
		},
	];

	return (
		<section className='border-t border-ink/10 px-6 py-24 md:py-40'>
			<div className='mx-auto max-w-5xl'>
				<div className='mb-12 max-w-3xl'>
					<motion.span
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className='mb-4 inline-block font-mono text-[11px] font-medium uppercase tracking-[0.18em] text-pine'
					>
						The Rest of the Suite
					</motion.span>
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.05 }}
						className='mb-4 text-3xl font-medium tracking-[-0.015em] text-ink md:text-4xl'
					>
						Pairs with agentOS.
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className='text-base leading-relaxed text-ink-soft md:text-lg'
					>
						agentOS is where agents live. Secure Exec is how you safely run the code they generate. Sandbox Agent SDK is how you control coding agents over HTTP.
					</motion.p>
				</div>

				<div className='grid grid-cols-1 gap-6 md:grid-cols-2'>
					{products.map((product, idx) => (
						<motion.a
							key={product.name}
							href={product.href}
							target='_blank'
							rel='noopener noreferrer'
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.05 * idx }}
							className='group flex flex-col rounded-xl border border-ink/10 bg-white/55 p-6 transition-colors hover:border-ink/25'
						>
							<h3 className='mb-2 text-lg font-medium text-ink'>{product.name}</h3>
							<p className='mb-6 text-sm leading-relaxed text-ink-soft'>{product.tagline}</p>
							<ul className='mb-8 flex flex-grow flex-col gap-2'>
								{product.bullets.map((bullet) => (
									<li key={bullet} className='flex items-start gap-2 text-sm leading-relaxed text-ink-soft'>
										<span className='mt-2 h-1 w-1 flex-shrink-0 rounded-full bg-ink/30' />
										<span>{bullet}</span>
									</li>
								))}
							</ul>
							<div className='inline-flex items-center gap-2 text-sm font-medium text-ink transition-colors group-hover:text-pine'>
								{product.cta}
								<ArrowRight className='h-3.5 w-3.5 transition-transform group-hover:translate-x-0.5' />
							</div>
						</motion.a>
					))}
				</div>
			</div>
		</section>
	);
};

const FromUnixToAgents = () => (
	<section className='border-t border-ink/10 px-6 py-24 md:py-40'>
		<div className='mx-auto max-w-5xl'>
			<div className='flex flex-col gap-10 md:flex-row md:items-center md:gap-16'>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className='flex-1'
				>
					<BeforeAfterSlider
						before='https://assets.rivet.dev/website/public/images/agent-os/unix-timesharing-uw-madison-1978.jpg'
						after='https://assets.rivet.dev/website/public/images/agent-os/data-flock.jpg'
					/>
					<p className='mt-2 font-mono text-xs text-ink-faint'>
						Left: Unix timesharing, UW-Madison, 1978. Right: "Data flock (digits)" by Philipp Schmitt, <a href='https://commons.wikimedia.org/wiki/File:Data_flock_(digits)_by_Philipp_Schmitt.jpg' className='underline hover:text-pine' target='_blank' rel='noopener noreferrer'>CC BY-SA 4.0</a>
					</p>
				</motion.div>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className='flex-1'
				>
					<h2 className='mb-4 text-3xl font-medium tracking-[-0.015em] text-ink md:text-4xl'>
						From humans to agents
					</h2>
					<p className='mb-6 text-base leading-relaxed text-ink-soft md:text-lg'>
						The operating system is changing for the next generation of software operators.
					</p>
					<a
						href='/from-unix-to-agents'
						className='selection-dark inline-flex items-center gap-2 rounded-md bg-ink px-4 py-2 text-sm font-medium text-cream transition-colors hover:bg-ink/85'
					>
						Learn more
						<ArrowRight className='h-4 w-4' />
					</a>
				</motion.div>
			</div>
		</div>
	</section>
);

// --- Main Page ---
export default function AgentOSPage({ heroTabs }: AgentOSPageProps) {
	return (
		<div className='paper-grain min-h-screen font-sans text-ink-soft' style={{ overflowX: 'clip' }}>
			<main>
				<Hero heroTabs={heroTabs} />
				<TechnologyAndBenchmarks />
				<AgentOSFeatures />
				<SisterProducts />
				<FromUnixToAgents />
			</main>
		</div>
	);
}

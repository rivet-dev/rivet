'use client';

import { useState, useEffect, useRef, useCallback } from 'react';
import {
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
} from 'lucide-react';
import { AnimatePresence, motion } from 'framer-motion';
import agentosLogo from '@/images/products/agentos-logo.svg';

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
		src: '/images/agent-os/division-classification-cataloging.jpg',
		title: 'Division of Classification and Cataloging',
		caption: 'Manual human labor at scale',
	},
	{
		src: '/images/agent-os/crowded-office-space.jpg',
		title: 'Crowded Office Space',
		caption: 'Rooms full of human operators',
	},
	// Automation with computers
	{
		src: '/images/agent-os/early-computer-room.jpg',
		title: 'Early Computer Room',
		caption: 'The first machines',
	},
	{
		src: '/images/agent-os/unix-timesharing-uw-madison-1978.jpg',
		title: 'Unix Timesharing',
		caption: 'UW-Madison, 1978',
	},
	{
		src: '/images/agent-os/early-computing-workstation.jpg',
		title: 'Early Computing Workstation',
		caption: 'Humans operating computers',
	},
	{
		src: '/images/agent-os/apollo-14-mission-control.jpg',
		title: 'Apollo 14: Mission Control Center',
		caption: 'Computers in mission-critical work',
	},
	// Modern work
	{
		src: '/images/agent-os/modern-office.jpg',
		title: 'Modern Office',
		caption: "Today's human operators",
	},
	// AI agents of tomorrow
	{
		src: '/images/agent-os/data-flock.jpg',
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
				<p className='text-sm font-medium text-zinc-900'>{currentImage.title}</p>
				<p className='text-xs text-zinc-500'>{currentImage.caption}</p>
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
			className='group inline-flex w-full items-center justify-center gap-2 whitespace-nowrap rounded-md border border-zinc-300 px-4 py-2 text-sm transition-colors hover:border-zinc-400 sm:w-auto'
		>
			<Terminal className='h-4 w-4 text-zinc-400' />
			<span className='text-zinc-700'>{command}</span>
			{copied && <Check className='h-4 w-4 text-emerald-500' />}
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
					fontFamily: '"Playwrite IE", cursive',
					fontSize: '72px',
					fontWeight: 400,
					fill: 'black',
					stroke: 'black',
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
	{ text: '$ npx agentos start', color: 'text-zinc-900', delay: 0, typing: true },
	{ text: '', delay: 600 },
	{ text: AGENTOS_ASCII, color: 'text-zinc-900', delay: 800 },
	{ text: '', delay: 1200 },
	{ text: '  v0.1.0  |  runtime ready', color: 'text-zinc-400', delay: 1400 },
	{ text: '', delay: 1600 },
	{ text: '✓ V8 isolate pool initialized (12 workers)', color: 'text-emerald-600', delay: 1800 },
	{ text: '✓ File system mounted → /workspace', color: 'text-emerald-600', delay: 2100 },
	{ text: '✓ Tool registry loaded (git, curl, python, node)', color: 'text-emerald-600', delay: 2400 },
	{ text: '✓ Network policy applied → allowlist mode', color: 'text-emerald-600', delay: 2700 },
	{ text: '', delay: 3000 },
	{ text: '● Agent session created  sid=a8f3c2e1', color: 'text-blue-600', delay: 3200 },
	{ text: '  → Claude Code connected', color: 'text-zinc-500', delay: 3500 },
	{ text: '  → Prompt: "Set up a Next.js app with auth"', color: 'text-zinc-500', delay: 3800 },
	{ text: '', delay: 4100 },
	{ text: '  ▸ agent  npm create next-app@latest /workspace/app', color: 'text-zinc-600', delay: 4400 },
	{ text: '  ▸ agent  npm install next-auth@5 prisma @prisma/client', color: 'text-zinc-600', delay: 5000 },
	{ text: '  ▸ agent  Writing 7 files...', color: 'text-zinc-600', delay: 5600 },
	{ text: '  ▸ agent  npx prisma db push', color: 'text-zinc-600', delay: 6200 },
	{ text: '', delay: 6800 },
	{ text: '✓ Task complete  duration=14.2s  tokens=3,847  cost=$0.012', color: 'text-emerald-600', delay: 7000 },
	{ text: '  → Preview: http://localhost:3000', color: 'text-zinc-500', delay: 7300 },
	{ text: '', delay: 7600 },
	{ text: '● Listening for new sessions...', color: 'text-blue-600', delay: 7800 },
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
		<div className='overflow-hidden rounded-xl border border-zinc-200 bg-white shadow-lg'>
			<div className='flex items-center gap-2 border-b border-zinc-100 px-4 py-3'>
				<div className='h-3 w-3 rounded-full bg-zinc-200' />
				<div className='h-3 w-3 rounded-full bg-zinc-200' />
				<div className='h-3 w-3 rounded-full bg-zinc-200' />
				<span className='ml-2 text-xs text-zinc-400'>terminal</span>
			</div>
			<div
				ref={scrollRef}
				className='h-[360px] overflow-y-auto p-4 font-mono text-[11px] leading-relaxed md:h-[420px] md:text-xs'
			>
				{terminalLines.slice(0, visibleCount).map((line, i) => (
					<div key={i} className={`${line.color || 'text-zinc-400'} whitespace-pre-wrap`}>
						{line.text || '\u00A0'}
					</div>
				))}
				{isTyping && (
					<div className={`${terminalLines[visibleCount]?.color || 'text-zinc-400'} whitespace-pre-wrap`}>
						{typedText}
						<span className='animate-pulse'>▌</span>
					</div>
				)}
				{visibleCount >= terminalLines.length && (
					<div className='text-zinc-400'>
						<span className='animate-pulse'>▌</span>
					</div>
				)}
			</div>
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
const heroTabMeta: Array<{ key: string; icon: typeof Bot; label: string; docsHref: string }> = [
	{ key: 'agents', icon: Bot, label: 'Agents', docsHref: '/docs/agent-os/sessions' },
	{ key: 'tools', icon: Wrench, label: 'Tools', docsHref: '/docs/agent-os/tools' },
	{ key: 's3-filesystem', icon: HardDrive, label: 'S3 File System', docsHref: '/docs/agent-os/filesystem' },
	{ key: 'cron', icon: CalendarClock, label: 'Cron', docsHref: '/docs/agent-os/cron' },
	{ key: 'webhooks', icon: Webhook, label: 'Webhooks', docsHref: '/docs/agent-os/webhooks' },
	{ key: 'multiplayer', icon: Users, label: 'Multiplayer', docsHref: '/docs/agent-os/multiplayer' },
	{ key: 'agent-agent', icon: Layers, label: 'Agent-Agent', docsHref: '/docs/agent-os/agent-to-agent' },
	{ key: 'workflows', icon: Workflow, label: 'Workflows', docsHref: '/docs/agent-os/workflows' },
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
		<section className='relative flex min-h-[100svh] flex-col justify-center px-6 pt-20 md:pt-0'>
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
						<span className='absolute -right-[8px] -top-[7px] rounded-full border border-zinc-900 bg-white px-2 py-0.5 text-[10px] font-medium text-zinc-900'>Beta</span>
					</div>
				</motion.div>

				{/* Subtitle */}
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					animate={{ opacity: 1, y: 0 }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className='mb-10 max-w-2xl text-center text-base text-zinc-500 md:text-left md:text-lg'
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
					<span className='text-xs text-zinc-400 uppercase tracking-wider'>Works with</span>
					<div className='flex flex-wrap items-center justify-center gap-2 md:justify-start md:gap-4'>
						{agents.map((agent) => (
							<div
								key={agent.name}
								className='flex cursor-pointer items-center gap-1.5 rounded-md px-2 py-1 transition-colors hover:bg-zinc-100'
								onMouseEnter={() => autoPlayComplete && setHoveredAgent(agent)}
								onMouseLeave={() => autoPlayComplete && setHoveredAgent(null)}
							>
								<img src={agent.src} alt={agent.name} className='h-4 w-4' />
								<span className='text-sm text-zinc-500'>{agent.name}{agent.comingSoon && '*'}</span>
							</div>
						))}
					</div>
					<span className='text-xs text-zinc-400'>*Coming Soon</span>
				</motion.div>

				{/* Code snippets */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					animate={{ opacity: 1, y: 0 }}
					transition={{ duration: 0.5, delay: 0.15 }}
				>
					{/* Tabs */}
					<div className='mb-4 overflow-x-auto pb-1'>
						<div className='flex min-w-max flex-nowrap items-center justify-start gap-1.5'>
						{getStartedTabs.map((tab, idx) => {
							const Icon = tab.icon;
							return (
								<button
									key={tab.label}
									onClick={() => setActiveTab(idx)}
									className='relative inline-flex shrink-0 items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs whitespace-nowrap transition-colors md:text-sm'
								>
									{activeTab === idx && (
										<motion.div
											layoutId='activeTab'
											className='absolute inset-0 rounded-lg bg-zinc-200'
											transition={{ type: 'spring', bounce: 0.2, duration: 0.4 }}
										/>
									)}
									<span className={`relative z-10 flex items-center gap-2 ${activeTab === idx ? 'text-zinc-900' : 'text-zinc-500 hover:text-zinc-700'}`}>
										<Icon className='h-4 w-4' />
										{tab.label}
									</span>
								</button>
							);
						})}
						</div>
					</div>

					{/* Code block */}
					<div className='overflow-hidden rounded-xl border border-zinc-200 bg-zinc-50'>
						<div className='flex items-center gap-2 border-b border-zinc-200 px-4 py-3'>
							<div className='h-3 w-3 rounded-full bg-zinc-200' />
							<div className='h-3 w-3 rounded-full bg-zinc-200' />
							<div className='h-3 w-3 rounded-full bg-zinc-200' />
							<span className='ml-2 text-xs text-zinc-600'>{getStartedTabs[activeTab]?.fileName ?? 'index.ts'}</span>
						</div>
						<div className='relative h-[380px] overflow-y-auto'>
							<AnimatePresence mode='wait'>
								<motion.div
									key={activeTab}
									initial={{ opacity: 0 }}
									animate={{ opacity: 1 }}
									exit={{ opacity: 0 }}
									transition={{ duration: 0.2 }}
									className='overflow-x-auto p-6 font-mono text-sm leading-relaxed text-zinc-600 [&_.line]:break-all [&_.shiki]:!m-0 [&_.shiki]:!bg-transparent [&_.shiki]:!p-0 [&_.shiki]:font-mono [&_.shiki]:text-sm [&_.shiki]:leading-relaxed'
								>
									<span
										className='not-prose code'
										// biome-ignore lint/security/noDangerouslySetInnerHtml: generated from shiki during Astro render
										dangerouslySetInnerHTML={{ __html: getStartedTabs[activeTab]?.highlightedCode ?? '' }}
									/>
								</motion.div>
							</AnimatePresence>
						</div>
					</div>
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
						className='selection-dark inline-flex w-full items-center justify-center gap-2 whitespace-nowrap rounded-md bg-zinc-900 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-zinc-700 sm:w-auto'
					>
						Read the Docs
						<ArrowRight className='h-4 w-4' />
					</a>
					<CopyCommand command='npm install rivetkit' />
					<div className='flex-1' />
					<a
						href='/agent-os/registry'
						className='inline-flex items-center gap-2 whitespace-nowrap text-sm text-zinc-500 transition-colors hover:text-zinc-900'
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
		className='border-t border-zinc-200 pt-6'
	>
		<div className='mb-3 text-zinc-500'>
			<IconComponent className='h-4 w-4' />
		</div>
		<h3 className='mb-2 text-base font-normal text-zinc-900'>
			{title}
		</h3>
		<p className='mb-4 text-sm leading-relaxed text-zinc-500'>{description}</p>
		{tags && (
			<div className='flex flex-wrap gap-2'>
				{tags.map((tag) => (
					<span
						key={tag}
						className='rounded bg-zinc-100 px-2.5 py-1 font-mono text-xs text-zinc-500'
					>
						{tag}
					</span>
				))}
			</div>
		)}
		{metric && (
			<div className='flex items-baseline gap-2'>
				<span className='font-mono text-3xl font-normal text-zinc-900'>
					{metric.value}
				</span>
				<span className='text-sm text-zinc-500'>{metric.label}</span>
			</div>
		)}
	</motion.div>
);

const DocsLink = ({ href }: { href: string }) => (
	<a
		href={href}
		className='inline-flex items-center gap-1 text-sm text-zinc-500 transition-colors hover:text-zinc-900'
	>
		Docs <span aria-hidden='true'>→</span>
	</a>
);

// --- Icon Box (rounded square outline like Rivet/agentOS logos) ---
const IconBox = ({ children }: { children: React.ReactNode }) => (
	<div className='relative mb-6 flex h-10 w-10 items-center justify-center md:h-12 md:w-12'>
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
		<section ref={sectionRef} className='border-t border-zinc-200'>
			{/* Fade gradient overlay at bottom - only show when section is in view */}
			{isInView && (
				<div
					className='pointer-events-none fixed bottom-0 left-0 right-0 z-20 h-64'
					style={{
						background: 'linear-gradient(to top, white 0%, white 20%, transparent 100%)',
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
					className='mx-auto max-w-4xl text-center text-3xl font-normal tracking-tight text-zinc-900 md:text-5xl'
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
										className='mb-6 flex min-h-0 flex-col rounded-2xl border border-zinc-200 bg-zinc-50 p-8 shadow-2xl md:p-12'
										style={{
											minHeight: `${CARD_HEIGHT - 24}px`,
											boxShadow: '0 -20px 60px rgba(0, 0, 0, 0.08), 0 -4px 30px rgba(0, 0, 0, 0.04)',
										}}
									>
										<IconBox>
											<Icon className='h-4 w-4 text-zinc-900 md:h-5 md:w-5' />
										</IconBox>
										<h2 className='mb-4 text-2xl font-normal tracking-tight text-zinc-900 md:text-4xl'>
											{feature.title}
										</h2>
										<p className='mb-4 max-w-2xl text-base leading-relaxed text-zinc-500 md:text-lg'>
											{feature.description}
										</p>
										{feature.detail && (
											<p className='mb-6 max-w-2xl text-sm leading-relaxed text-zinc-500 md:text-base'>
												{feature.detail}
											</p>
										)}
										{feature.tags && (
											<div className='mb-4 flex flex-wrap gap-2'>
												{feature.tags.map((tag) => (
													<span
														key={tag}
														className='rounded-full border border-zinc-200 bg-zinc-100 px-4 py-1.5 font-mono text-sm text-zinc-500'
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
														<span className='font-mono text-5xl font-normal text-zinc-900 md:text-7xl'>
															{m.value}
														</span>
														<span className='mt-2 text-sm text-zinc-500 md:text-base'>{m.label}</span>
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
				className='-mx-6 flex gap-4 overflow-x-auto px-6 pb-4 scrollbar-none'
				style={{ scrollbarWidth: 'none', msOverflowStyle: 'none' }}
			>
				{section.features.map((feature) => {
					const Icon = feature.icon;
						return (
							<div
								key={feature.title}
								className='relative flex w-[280px] flex-shrink-0 flex-col rounded-2xl bg-zinc-50 p-6'
							>
							{feature.comingSoon && (
								<span className='absolute top-4 right-4 rounded-full bg-amber-100 px-2 py-0.5 text-[10px] font-medium text-amber-700'>
									Coming Soon
								</span>
							)}
							<div className='mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-200/60'>
								<Icon className='h-5 w-5 text-zinc-600' />
							</div>
							<h3 className='mb-2 text-sm font-semibold text-zinc-900'>
								{feature.title}
							</h3>
								<p className='text-sm leading-relaxed text-zinc-500'>
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
							? 'border-zinc-300 text-zinc-900 hover:bg-zinc-100'
							: 'border-zinc-200 text-zinc-300 cursor-default'
					}`}
				>
					<ChevronLeft className='h-4 w-4' />
				</button>
				<button
					onClick={() => scroll('right')}
					disabled={!canScrollRight}
					className={`flex h-9 w-9 items-center justify-center rounded-xl border transition-colors ${
						canScrollRight
							? 'border-zinc-300 text-zinc-900 hover:bg-zinc-100'
							: 'border-zinc-200 text-zinc-300 cursor-default'
					}`}
				>
					<ChevronRight className='h-4 w-4' />
				</button>
			</div>
		</div>
	);
};

const ThemedFeatureSections = () => (
	<div className='mt-32 md:mt-48'>
		{themedSections.map((section) => (
			<section
				key={section.category}
				className='border-t border-zinc-200 px-6 py-24 md:py-40'
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
						<h2 className='mb-4 text-3xl font-normal tracking-tight text-zinc-900 md:text-5xl lg:text-6xl'>
							{section.title}
						</h2>
						<p className='max-w-xl text-base text-zinc-500 md:text-lg'>
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
	<section className='border-t border-zinc-200 px-6 py-24 md:py-40'>
		<div className='mx-auto max-w-7xl'>
			<motion.div
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className='rounded-xl border border-zinc-200 bg-zinc-50 p-8 md:p-12'
			>
				<div className='flex flex-col items-start gap-6 md:flex-row md:items-center md:justify-between'>
					<div>
						<h3 className='mb-2 text-2xl font-normal tracking-tight text-zinc-900 md:text-3xl'>
							agentOS Registry
						</h3>
						<p className='max-w-lg text-base leading-relaxed text-zinc-500'>
							Browse and install pre-built tools, integrations, and capabilities for your agents. From file systems to databases to API connectors.
						</p>
					</div>
					<a
						href='/agent-os/registry'
						className='selection-dark inline-flex flex-shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-md bg-zinc-900 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-zinc-700'
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
const BENCH_ACCENT = '#18181b';
const BENCH_ACCENT_LIGHT = '#3f3f46';

// Benchmark data (computed from raw inputs in bench.ts)
import { benchColdStart, benchWorkloads, SANDBOX_COLDSTART_PROVIDER, SANDBOX_COST_PROVIDER, type WorkloadKey } from '@/data/bench';

function BenchInfoTooltip({ children }: { children: React.ReactNode }) {
	return (
		<span className='group/tip relative ml-1.5 inline-flex align-middle'>
			<svg
				className='h-3.5 w-3.5 cursor-help text-zinc-600 transition-colors group-hover/tip:text-zinc-500'
				viewBox='0 0 16 16'
				fill='currentColor'
			>
				<path d='M8 0a8 8 0 100 16A8 8 0 008 0zm1 12H7V7h2v5zm-1-6a1 1 0 110-2 1 1 0 010 2z' />
			</svg>
			<span className='pointer-events-none absolute bottom-full left-0 z-50 mb-2 w-80 rounded-lg border border-zinc-200 bg-white/95 p-3 text-[11px] leading-relaxed text-zinc-600 opacity-0 shadow-xl backdrop-blur-sm transition-opacity duration-200 group-hover/tip:pointer-events-auto group-hover/tip:opacity-100 [&_a]:text-zinc-900 [&_a]:underline [&_a]:underline-offset-2 [&_strong]:font-medium [&_strong]:text-zinc-800'>
				{children}
			</span>
		</span>
	);
}

function BenchColdStartChart() {
	const groups = benchColdStart;
	const [active, setActive] = useState(2);
	const g = groups[active];
	const pct = Math.max((g.agentOS / g.sandbox) * 100, 1);

	return (
		<motion.div
			className='space-y-4'
			initial='hidden'
			whileInView='visible'
			viewport={{ once: true, margin: '-100px' }}
		>
			<div className='flex items-center gap-4'>
				<div>
					<h4 className='flex items-center text-sm font-medium text-zinc-900'>
						Cold start
						<BenchInfoTooltip>
							<strong>What&apos;s measured:</strong> Time from requesting an execution to first code running.
							<br /><br />
							<strong>Why the gap:</strong> agentOS boots a lightweight VM inside the host process. No network hop, no disk image. Sandboxes must boot an entire environment, allocate memory, and establish a network connection before code can run.
							<br /><br />
							<strong>Sandbox baseline:</strong> {SANDBOX_COLDSTART_PROVIDER}, the fastest mainstream sandbox provider as of March 30, 2026.
							<br /><br />
							<strong>agentOS:</strong> Median of 10,000 runs (100 iterations x 100 samples) on Intel i7-12700KF.
						</BenchInfoTooltip>
					</h4>
					<p className='mt-1 text-[11px] italic text-zinc-600'>Lower is better</p>
				</div>
				<div className='ml-auto flex gap-1'>
					{groups.map((t, i) => (
						<button
							key={t.label}
							onClick={() => setActive(i)}
							className={`rounded px-2.5 py-1 font-mono text-[11px] uppercase tracking-wider transition-colors ${
								i === active ? 'bg-zinc-200 text-zinc-900' : 'text-zinc-600 hover:text-zinc-500'
							}`}
						>
							{t.label}
						</button>
					))}
				</div>
			</div>
			<div className='space-y-1.5'>
				<div className='flex items-center gap-4'>
					<span className='w-48 shrink-0 font-mono text-xs text-zinc-500'>agentOS</span>
					<div className='relative h-7 flex-1 overflow-hidden rounded-sm bg-zinc-100'>
						<motion.div
							key={`coldstart-${active}`}
							initial={{ width: 0 }}
							animate={{ width: `${pct}%` }}
							transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1] }}
							className='absolute inset-y-0 left-0 rounded-sm'
							style={{ background: `linear-gradient(90deg, ${BENCH_ACCENT}, ${BENCH_ACCENT_LIGHT})` }}
						/>
						<motion.span
							key={`label-coldstart-${active}`}
							initial={{ opacity: 0 }}
							animate={{ opacity: 1 }}
							transition={{ duration: 0.4, delay: 0.5 }}
							className='absolute inset-y-0 z-10 flex items-center gap-2 font-mono text-xs font-medium text-zinc-500'
							style={{ left: `calc(${pct}% + 8px)` }}
						>
							{g.agentOS} ms
							<span className='text-[11px] font-semibold' style={{ color: BENCH_ACCENT_LIGHT }}>
								{Math.round(g.sandbox / g.agentOS)}x faster
							</span>
						</motion.span>
					</div>
				</div>
				<div className='flex items-center gap-4'>
					<span className='w-48 shrink-0 font-mono text-xs text-zinc-500'>Fastest sandbox</span>
					<div className='relative h-7 flex-1 overflow-hidden rounded-sm bg-zinc-100'>
						<motion.div
							key={`coldstart-sandbox-${active}`}
							initial={{ width: 0 }}
							animate={{ width: '100%' }}
							transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1], delay: 0.1 }}
							className='absolute inset-y-0 left-0 rounded-sm bg-zinc-400'
						/>
						<motion.span
							key={`sandbox-label-coldstart-${active}`}
							initial={{ opacity: 0 }}
							animate={{ opacity: 1 }}
							transition={{ duration: 0.4, delay: 0.6 }}
							className='absolute inset-y-0 left-2 z-10 flex items-center font-mono text-xs text-zinc-700'
						>
							{g.sandbox.toLocaleString()} ms
						</motion.span>
					</div>
				</div>
			</div>
		</motion.div>
	);
}

function BenchMemoryBar({ workload }: { workload: WorkloadKey }) {
	const mem = benchWorkloads[workload].memory;
	const barMin = Math.max(mem.agentOSBar, 1);
	return (
		<motion.div
			className='space-y-4'
			initial='hidden'
			whileInView='visible'
			viewport={{ once: true, margin: '-100px' }}
		>
			<div>
				<h4 className='flex items-center text-sm font-medium text-zinc-900'>
					Memory per instance
					<BenchInfoTooltip>
						<strong>What&apos;s measured:</strong> Memory footprint added per concurrent execution.
						<br /><br />
						<strong>Why the gap:</strong> Lightweight VMs share the host process. Each additional execution only adds its own heap and stack. Sandboxes allocate a dedicated environment with a minimum memory reservation, even if the code inside uses far less.
						<br /><br />
						<strong>Sandbox baseline:</strong> {SANDBOX_COST_PROVIDER}, the cheapest mainstream sandbox provider as of March 30, 2026. Default sandbox: 1 vCPU + 1 GiB RAM.
						<br /><br />
						<strong>agentOS:</strong> {workload === 'agent' ? `${benchWorkloads.agent.memory.agentOS} for a full Pi coding agent session with MCP servers and file system mounts.` : `${benchWorkloads.shell.memory.agentOS} for the minimal shell workload under sustained load.`}
					</BenchInfoTooltip>
				</h4>
				<p className='mt-1 text-[11px] italic text-zinc-600'>Lower is better. Sandboxes reserve idle RAM per agent.</p>
			</div>
			<div className='space-y-1.5'>
				<div className='flex items-center gap-4'>
					<span className='w-48 shrink-0 font-mono text-xs text-zinc-500'>agentOS</span>
					<div className='relative h-7 flex-1 overflow-hidden rounded-sm bg-zinc-100'>
						<motion.div
							key={workload}
							initial={{ width: 0 }}
							animate={{ width: `${barMin}%` }}
							transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1] }}
							className='absolute inset-y-0 left-0 rounded-sm'
							style={{ background: `linear-gradient(90deg, ${BENCH_ACCENT}, ${BENCH_ACCENT_LIGHT})` }}
						/>
						<motion.span
							key={`mem-label-${workload}`}
							initial={{ opacity: 0 }}
							animate={{ opacity: 1 }}
							transition={{ duration: 0.4, delay: 0.5 }}
							className='absolute inset-y-0 z-10 flex items-center gap-2 font-mono text-xs font-medium text-zinc-500'
							style={{ left: `calc(${barMin}% + 8px)` }}
						>
							{mem.agentOS}
							<span className='text-[11px] font-semibold' style={{ color: BENCH_ACCENT_LIGHT }}>
								{mem.multiplier}
							</span>
						</motion.span>
					</div>
				</div>
				<div className='flex items-center gap-4'>
					<span className='w-48 shrink-0 font-mono text-xs text-zinc-500'>Cheapest sandbox</span>
					<div className='relative h-7 flex-1 overflow-hidden rounded-sm bg-zinc-100'>
						<motion.div
							initial={{ width: 0 }}
							whileInView={{ width: `${mem.sandboxBar}%` }}
							viewport={{ once: true }}
							transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1], delay: 0.1 }}
							className='absolute inset-y-0 left-0 rounded-sm bg-zinc-400'
						/>
						<motion.span
							initial={{ opacity: 0 }}
							whileInView={{ opacity: 1 }}
							viewport={{ once: true }}
							transition={{ duration: 0.4, delay: 0.6 }}
							className='absolute inset-y-0 left-2 z-10 flex items-center font-mono text-xs text-zinc-700'
						>
							{mem.sandbox}
						</motion.span>
					</div>
				</div>
			</div>
		</motion.div>
	);
}

function BenchCostChart({ workload }: { workload: WorkloadKey }) {
	const tiers = benchWorkloads[workload].cost;
	const sandboxCost = benchWorkloads[workload].sandboxCost;
	const [active, setActive] = useState(0);
	const t = tiers[active];
	const barMin = Math.max(t.bar, 1);

	return (
		<motion.div
			className='space-y-4'
			initial='hidden'
			whileInView='visible'
			viewport={{ once: true, margin: '-100px' }}
		>
			<div className='flex items-center gap-4'>
				<div>
					<h4 className='flex items-center text-sm font-medium text-zinc-900'>
						Cost per execution-second
						<BenchInfoTooltip>
							<strong>What&apos;s measured:</strong> <code className='rounded bg-zinc-200 px-1 py-0.5 text-[10px]'>server price per second / concurrent executions per server</code>
							<br /><br />
							<strong>Why it&apos;s cheaper:</strong> Each execution uses {benchWorkloads[workload].memory.agentOS} instead of a {benchWorkloads[workload].memory.sandbox} sandbox minimum. And you run on your own hardware, which is significantly cheaper than per-second sandbox billing.
							<br /><br />
							<strong>Sandbox baseline:</strong> {SANDBOX_COST_PROVIDER}, the cheapest mainstream sandbox provider as of March 30, 2026. Default sandbox: 1 vCPU + 1 GiB RAM at $0.0504/vCPU-h + $0.0162/GiB-h.
							<br /><br />
							<strong>agentOS:</strong> {benchWorkloads[workload].memory.agentOS} baseline per execution, assuming 70% utilization (industry-standard HPA scaling threshold). Select a hardware tier above to compare.
						</BenchInfoTooltip>
					</h4>
					<p className='mt-1 text-[11px] italic text-zinc-600'>Lower is better. Assumes one agent per sandbox, needed for isolation.</p>
				</div>
				<div className='ml-auto flex gap-1'>
					{tiers.map((tier, i) => (
						<button
							key={tier.label}
							onClick={() => setActive(i)}
							className={`rounded px-2.5 py-1 font-mono text-[11px] tracking-wider transition-colors ${
								i === active ? 'bg-zinc-200 text-zinc-900' : 'text-zinc-600 hover:text-zinc-500'
							}`}
						>
							{tier.label}
						</button>
					))}
				</div>
			</div>
			<div className='space-y-1.5'>
				<div className='flex items-center gap-4'>
					<span className='w-48 shrink-0 font-mono text-xs text-zinc-500'>agentOS</span>
					<div className='relative h-7 flex-1 overflow-hidden rounded-sm bg-zinc-100'>
						<motion.div
							key={`${workload}-${active}`}
							initial={{ width: 0 }}
							animate={{ width: `${barMin}%` }}
							transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1] }}
							className='absolute inset-y-0 left-0 rounded-sm'
							style={{ background: `linear-gradient(90deg, ${BENCH_ACCENT}, ${BENCH_ACCENT_LIGHT})` }}
						/>
						<motion.span
							key={`cost-label-${workload}-${active}`}
							initial={{ opacity: 0 }}
							animate={{ opacity: 1 }}
							transition={{ duration: 0.4, delay: 0.5 }}
							className='absolute inset-y-0 z-10 flex items-center gap-2 font-mono text-xs font-medium text-zinc-500'
							style={{ left: `calc(${barMin}% + 8px)` }}
						>
							{t.value}
							<span className='text-[11px] font-semibold' style={{ color: BENCH_ACCENT_LIGHT }}>
								{t.multiplier}
							</span>
						</motion.span>
					</div>
				</div>
				<div className='flex items-center gap-4'>
					<span className='w-48 shrink-0 font-mono text-xs text-zinc-500'>Cheapest sandbox</span>
					<div className='relative h-7 flex-1 overflow-hidden rounded-sm bg-zinc-100'>
						<motion.div
							key={`${workload}-${active}`}
							initial={{ width: 0 }}
							animate={{ width: '100%' }}
							transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1], delay: 0.1 }}
							className='absolute inset-y-0 left-0 rounded-sm bg-zinc-400'
						/>
						<motion.span
							key={`sandbox-cost-${workload}-${active}`}
							initial={{ opacity: 0 }}
							animate={{ opacity: 1 }}
							transition={{ duration: 0.4, delay: 0.6 }}
							className='absolute inset-y-0 left-2 z-10 flex items-center font-mono text-xs text-zinc-700'
						>
							{sandboxCost}
						</motion.span>
					</div>
				</div>
			</div>
		</motion.div>
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
				<h3 className='mb-2 text-2xl font-normal tracking-tight text-zinc-900 md:text-3xl'>
					Performance benchmarks
				</h3>
				<p className='text-base leading-relaxed text-zinc-500'>
					agentOS vs. traditional sandboxes.
				</p>
			</div>

			<div className='rounded-xl border border-zinc-200 bg-zinc-50 p-8'>
				<BenchColdStartChart />
				<div className='my-8 border-t border-zinc-100' />
				<div className='mb-4 flex items-center justify-between'>
					<p className='text-xs text-zinc-400'>Workload: {wl.description}</p>
					<div className='flex gap-1 rounded-lg border border-zinc-200 bg-white p-1'>
						{(Object.keys(benchWorkloads) as WorkloadKey[]).map((key) => (
							<button
								key={key}
								onClick={() => setWorkload(key)}
								className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
									workload === key ? 'bg-zinc-900 text-white' : 'text-zinc-500 hover:text-zinc-700'
								}`}
							>
								{benchWorkloads[key].label}
							</button>
						))}
					</div>
				</div>
				<BenchMemoryBar workload={workload} />
				<div className='my-8 border-t border-zinc-100' />
				<BenchCostChart workload={workload} />
			</div>

			<p className='mt-4 text-[11px] leading-relaxed text-zinc-400'>
				Measured on Intel i7-12700KF. Cold start baseline: {SANDBOX_COLDSTART_PROVIDER}, the fastest mainstream sandbox provider as of March 30, 2026. Cost baseline: {SANDBOX_COST_PROVIDER}, the cheapest mainstream sandbox provider as of March 30, 2026 (1 vCPU + 1 GiB default). Cost assumes 70% utilization on self-hosted hardware vs. per-second sandbox billing.{' '}
				<a
					href='/docs/agent-os/benchmarks'
					className='inline-flex items-center gap-1 text-zinc-500 underline underline-offset-2 transition-colors hover:text-zinc-700'
				>
					Benchmark document
					<ExternalLink className='h-3 w-3' />
				</a>
			</p>
		</motion.div>
	);
}

const TechnologyAndBenchmarks = () => (
	<section className='border-t border-zinc-200 py-16 md:py-32'>
		<div className='mx-auto max-w-5xl px-6'>
			{/* Technology intro */}
			<motion.div
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className='mb-16'
			>
				<h2 className='mb-4 text-3xl font-normal tracking-tight text-zinc-900 md:text-5xl'>
					A new operating system architecture.
				</h2>
				<p className='mb-6 max-w-3xl text-base leading-relaxed text-zinc-500 md:text-lg'>
					Built from the ground up for lightweight agents. agentOS provides the flexibility of Linux with lower overhead than sandboxes.
				</p>
				<div className='grid gap-6 md:grid-cols-2'>
					<div className='rounded-xl border border-zinc-200 bg-zinc-50 p-6'>
						<div className='mb-3 flex items-center gap-3'>
							<div className='flex h-10 w-10 items-center justify-center rounded-lg bg-zinc-200'>
								<img src='/images/agent-os/webassembly-logo.svg' alt='WebAssembly' className='h-6 w-6 grayscale opacity-70' />
							</div>
							<h3 className='text-lg font-medium text-zinc-900'>WebAssembly + V8 Isolates</h3>
						</div>
						<p className='text-sm leading-relaxed text-zinc-500'>
							High-performance virtualization without specialized infrastructure. The same battle-hardened isolation technology that powers Google Chrome.
						</p>
					</div>
					<div className='rounded-xl border border-zinc-200 bg-zinc-50 p-6'>
						<div className='mb-3 flex items-center gap-3'>
							<div className='flex h-10 w-10 items-center justify-center rounded-lg bg-zinc-200'>
								<Globe className='h-5 w-5 text-zinc-700' />
							</div>
							<h3 className='text-lg font-medium text-zinc-900'>Battle-tested technology</h3>
						</div>
						<p className='text-sm leading-relaxed text-zinc-500'>
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
					<svg width='16' height='16' viewBox='0 0 16 16' fill='none'><path d='M5 3L2 8L5 13M11 3L14 8L11 13' stroke='#18181b' strokeWidth='1.5' strokeLinecap='round' strokeLinejoin='round'/></svg>
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
const FromUnixToAgents = () => (
	<section className='border-t border-zinc-200 px-6 py-24 md:py-40'>
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
						before='/images/agent-os/unix-timesharing-uw-madison-1978.jpg'
						after='/images/agent-os/data-flock.jpg'
					/>
					<p className='mt-2 text-xs text-zinc-400'>
						Left: Unix timesharing, UW-Madison, 1978. Right: "Data flock (digits)" by Philipp Schmitt, <a href='https://commons.wikimedia.org/wiki/File:Data_flock_(digits)_by_Philipp_Schmitt.jpg' className='underline hover:text-zinc-600' target='_blank' rel='noopener noreferrer'>CC BY-SA 4.0</a>
					</p>
				</motion.div>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className='flex-1'
				>
					<h2 className='mb-4 text-3xl font-normal tracking-tight text-zinc-900 md:text-4xl'>
						From humans to agents
					</h2>
					<p className='mb-6 text-base leading-relaxed text-zinc-500 md:text-lg'>
						The operating system is changing for the next generation of software operators.
					</p>
					<a
						href='/from-unix-to-agents'
						className='inline-flex items-center gap-2 rounded-md bg-zinc-900 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-zinc-700'
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
		<div className='min-h-screen bg-white font-sans text-zinc-600 selection:bg-zinc-200 selection:text-zinc-900'>
			<main>
				<Hero heroTabs={heroTabs} />
				<TechnologyAndBenchmarks />
				<AgentOSFeatures />
				<FromUnixToAgents />
			</main>
		</div>
	);
}

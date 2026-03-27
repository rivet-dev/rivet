"use client";

import {
	ArrowRight,
	Shield,
	Terminal,
	FolderOpen,
	Clock,
	Layers,
	Globe,
} from "lucide-react";
import { motion } from "framer-motion";
import agentosLogo from "@/images/products/agentos-logo.svg";

// --- Hero ---
const Hero = () => (
	<section className="relative flex min-h-[70svh] flex-col items-center justify-center px-6">
		<div className="mx-auto max-w-4xl text-center">
			<motion.h1
				initial={{ opacity: 0, y: 20 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{ duration: 0.5 }}
				className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl"
			>
				From human operators
				<br />
				to agent operators
			</motion.h1>
			<motion.p
				initial={{ opacity: 0, y: 20 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{ duration: 0.5, delay: 0.1 }}
				className="text-lg leading-relaxed text-zinc-400 md:text-xl"
			>
				Unix gave humans a common language to control machines.
				<br />
				AgentOS gives agents the same power.
			</motion.p>
		</div>
	</section>
);

// --- Timeline Era ---
interface EraProps {
	year: string;
	title: string;
	lead: string;
	body?: string;
	children?: React.ReactNode;
	future?: boolean;
	delay?: number;
}

const Era = ({ year, title, lead, body, children, future, delay = 0 }: EraProps) => (
	<motion.div
		initial={{ opacity: 0, y: 30 }}
		whileInView={{ opacity: 1, y: 0 }}
		viewport={{ once: true }}
		transition={{ duration: 0.6, delay }}
		className="grid grid-cols-1 gap-6 md:grid-cols-[100px_1fr] md:gap-12"
	>
		{/* Marker */}
		<div className="flex items-start gap-4 md:flex-col md:items-center">
			<span
				className={`font-mono text-sm font-semibold ${future ? "text-white" : "text-zinc-500"}`}
			>
				{year}
			</span>
			<div
				className={`hidden h-full w-px md:block ${future ? "bg-white" : "bg-white/10"}`}
			/>
		</div>

		{/* Content */}
		<div className="pb-16">
			<h2
				className={`mb-4 font-extrabold tracking-tight text-white ${future ? "text-3xl md:text-4xl" : "text-2xl md:text-3xl"}`}
			>
				{title}
			</h2>
			<p className="mb-4 text-base leading-relaxed text-zinc-400 md:text-lg">
				{lead}
			</p>
			{body && (
				<p className="mb-6 text-sm leading-relaxed text-zinc-500 md:text-base">
					{body}
				</p>
			)}
			{children}
		</div>
	</motion.div>
);

// --- Principle Chip ---
const PrincipleChip = ({ label, text }: { label: string; text: string }) => (
	<div className="rounded-lg border border-white/5 bg-white/[0.02] p-5">
		<span className="mb-2 block font-mono text-[11px] font-semibold uppercase tracking-wider text-zinc-500">
			{label}
		</span>
		<p className="text-sm leading-relaxed text-zinc-400">{text}</p>
	</div>
);

// --- Stat ---
const Stat = ({ number, label }: { number: string; label: string }) => (
	<div className="rounded-lg border border-white/5 bg-white/[0.02] p-6 text-center">
		<span className="block text-3xl font-extrabold tracking-tight text-white">
			{number}
		</span>
		<span className="mt-1 block text-xs text-zinc-500">{label}</span>
	</div>
);

// --- Timeline Section ---
const Timeline = () => (
	<section className="border-t border-white/10 py-16 md:py-24">
		<div className="mx-auto max-w-5xl px-6">
			{/* Unix */}
			<Era
				year="1969"
				title="The Unix Foundation"
				lead="Before Unix, every computer spoke a different language. Programs written for one machine couldn't run on another. Computing was fragmented, expensive, and inaccessible."
				body="Unix changed everything. It introduced a radical idea: a portable operating system with a universal interface. Files, processes, pipes, permissions. Simple primitives that composed into infinite complexity."
			>
				<div className="mb-6 overflow-hidden rounded-lg border border-white/5">
					<img
						src="/images/agent-os/ken-thompson-dennis-ritchie-1973.jpg"
						alt="Ken Thompson and Dennis Ritchie, creators of Unix, 1973"
						className="w-full object-cover opacity-70"
						loading="lazy"
					/>
					<p className="bg-white/[0.02] px-4 py-2 text-xs text-zinc-600">
						Ken Thompson and Dennis Ritchie, 1973.{" "}
						<a
							href="https://commons.wikimedia.org/w/index.php?curid=31308"
							className="underline hover:text-zinc-400"
							target="_blank"
							rel="noopener noreferrer"
						>
							Public Domain
						</a>
					</p>
				</div>
				<div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
					<PrincipleChip
						label="Philosophy"
						text="Do one thing well. Compose small programs into larger systems."
					/>
					<PrincipleChip
						label="Interface"
						text="Everything is a file. Text streams connect all programs."
					/>
					<PrincipleChip
						label="Impact"
						text="The foundation for Linux, macOS, Android, and the modern internet."
					/>
				</div>
			</Era>

			{/* Linux */}
			<Era
				year="1991"
				title="Linux & The Open Source Revolution"
				lead="Linux took Unix's ideas and made them free. Not just free as in cost. Free as in freedom. Anyone could read, modify, and distribute the code that powered their machines."
				body="This openness sparked an explosion of innovation. The kernel became the backbone of servers, phones, cars, and spacecraft. Open source became the default way to build software."
				delay={0.1}
			>
				<div className="mt-4 overflow-hidden rounded-lg border border-white/5">
					<img
						src="/images/agent-os/first-web-server.jpg"
						alt="The first web server at CERN"
						className="w-full object-cover opacity-70"
						loading="lazy"
					/>
					<p className="bg-white/[0.02] px-4 py-2 text-xs text-zinc-600">
						The first web server at CERN. Photo by Coolcaesar,{" "}
						<a
							href="https://commons.wikimedia.org/w/index.php?curid=395096"
							className="underline hover:text-zinc-400"
							target="_blank"
							rel="noopener noreferrer"
						>
							CC BY-SA 3.0
						</a>
					</p>
				</div>
			</Era>

			{/* Cloud */}
			<Era
				year="2006"
				title="The Cloud Era"
				lead="AWS, then Azure, then GCP. Computing became a utility. No more buying servers. Just rent capacity by the hour. Infrastructure as code. Scale on demand."
				body="But the fundamental model stayed the same: humans writing code, humans operating systems, humans in the loop at every step. The cloud made computing elastic, but it was still computing for humans."
				delay={0.2}
			>
				<div className="mb-6 overflow-hidden rounded-lg border border-white/5">
					<img
						src="/images/agent-os/nersc-server-racks.jpg"
						alt="Server racks at NERSC"
						className="w-full object-cover opacity-70"
						loading="lazy"
					/>
					<p className="bg-white/[0.02] px-4 py-2 text-xs text-zinc-600">
						Server racks at NERSC. Photo by Derrick Coetzee,{" "}
						<a
							href="https://commons.wikimedia.org/w/index.php?curid=17445617"
							className="underline hover:text-zinc-400"
							target="_blank"
							rel="noopener noreferrer"
						>
							CC0
						</a>
					</p>
				</div>
				<div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
					<PrincipleChip
						label="Model"
						text="Pay for what you use. Scale infinitely. APIs for everything."
					/>
					<PrincipleChip
						label="Assumption"
						text="Humans write the code. Humans click the buttons. Humans fix the errors."
					/>
				</div>
			</Era>

			{/* Agent Era */}
			<Era
				year="Now"
				title="The Agent Era"
				lead="AI agents are the new operators. They write code, run commands, fix errors, and deploy software. They work around the clock. They scale to thousands of instances. They don't need a GUI."
				body="But agents have different needs than humans. They need persistent memory that survives crashes. They need secure execution environments they can't escape. They need real-time communication with other agents and systems."
				future
			>
				<div className="mb-6 overflow-hidden rounded-lg border border-white/5">
					<img
						src="/images/agent-os/data-flock.jpg"
						alt="Data flock (digits) by Philipp Schmitt"
						className="w-full object-cover opacity-70"
						loading="lazy"
					/>
					<p className="bg-white/[0.02] px-4 py-2 text-xs text-zinc-600">
						"Data flock (digits)" by Philipp Schmitt,{" "}
						<a
							href="https://commons.wikimedia.org/wiki/File:Data_flock_(digits)_by_Philipp_Schmitt.jpg"
							className="underline hover:text-zinc-400"
							target="_blank"
							rel="noopener noreferrer"
						>
							CC BY-SA 4.0
						</a>
					</p>
				</div>
				<motion.p
					initial={{ opacity: 0 }}
					whileInView={{ opacity: 1 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
					className="mb-8 text-lg font-semibold text-white"
				>
					They need an operating system built for them.
				</motion.p>

				{/* Bar chart */}
				<div className="rounded-lg border border-white/5 bg-white/[0.02] p-6">
					<div className="mb-4 flex gap-3">
						<div className="h-6 flex-1 rounded bg-zinc-800" />
						<div className="h-6 flex-[3] rounded bg-white" />
					</div>
					<div className="flex justify-between text-xs text-zinc-500">
						<span>Human operators</span>
						<span>AI agents</span>
					</div>
					<p className="mt-3 text-sm text-zinc-500">
						Soon, more computing tasks will be performed by AI agents than
						by human operators.
					</p>
				</div>
			</Era>
		</div>
	</section>
);

// --- The Shift ---
const ShiftSection = () => (
	<section className="border-y border-white/10 bg-white/[0.02] py-20 md:py-28">
		<div className="mx-auto max-w-3xl px-6 text-center">
			<motion.h2
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className="mb-6 text-3xl font-extrabold tracking-tight text-white md:text-5xl"
			>
				The shift is happening now
			</motion.h2>
			<motion.p
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5, delay: 0.1 }}
				className="mb-4 text-base leading-relaxed text-zinc-400 md:text-lg"
			>
				For fifty years, we built operating systems for human operators. Humans
				who read documentation. Humans who type commands. Humans who understand
				error messages and fix bugs.
			</motion.p>
			<motion.p
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5, delay: 0.2 }}
				className="text-lg font-semibold text-white md:text-xl"
			>
				But the next wave of computing won't be operated by humans.
			</motion.p>
		</div>
	</section>
);

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
		className="rounded-lg border border-white/10 bg-white/[0.02] p-6 transition-colors hover:border-white/20"
	>
		<div className="mb-4 flex h-10 w-10 items-center justify-center rounded-lg border border-white/10 bg-white/5">
			<IconComponent className="h-5 w-5 text-white" />
		</div>
		<h3 className="mb-2 text-lg font-bold tracking-tight text-white">
			{title}
		</h3>
		<p className="mb-4 text-sm leading-relaxed text-zinc-400">{description}</p>
		{tags && (
			<div className="flex flex-wrap gap-2">
				{tags.map((tag) => (
					<span
						key={tag}
						className="rounded bg-white/5 px-2.5 py-1 font-mono text-xs text-zinc-500"
					>
						{tag}
					</span>
				))}
			</div>
		)}
		{metric && (
			<div className="flex items-baseline gap-2">
				<span className="font-mono text-3xl font-bold text-white">
					{metric.value}
				</span>
				<span className="text-sm text-zinc-500">{metric.label}</span>
			</div>
		)}
	</motion.div>
);

// --- AgentOS Features Section ---
const AgentOSFeatures = () => (
	<section id="agentos" className="border-t border-white/10 py-24 md:py-32">
		<div className="mx-auto max-w-7xl px-6">
			{/* Header */}
			<div className="mb-6 text-center">
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-4 font-mono text-xs font-semibold uppercase tracking-widest text-zinc-500"
				>
					Introducing
				</motion.p>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.05 }}
					className="mb-6 flex items-center justify-center gap-4"
				>
					<img
						src={agentosLogo.src}
						alt="AgentOS"
						className="h-14 w-14 md:h-16 md:w-16"
					/>
					<h2 className="text-5xl font-extrabold tracking-tight text-white md:text-7xl">
						AgentOS
					</h2>
				</motion.div>
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className="text-lg text-zinc-500 md:text-xl"
				>
					A lightweight Linux-like VM for agents.
					<br />
					Secured by V8 isolates and WebAssembly.
				</motion.p>
			</div>

			{/* Install block */}
			<motion.div
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5, delay: 0.15 }}
				className="mb-16 text-center"
			>
				<code className="inline-block rounded-lg border border-white/10 bg-white/5 px-8 py-4 font-mono text-base text-zinc-300">
					npm install @rivetkit/agent-os
				</code>
			</motion.div>

			{/* Features grid */}
			<div className="mb-16 grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
				<FeatureCard
					icon={Terminal}
					title="Your tools, ready to go"
					description="Git, curl, Python, npm. The tools agents already know. No custom runtimes or language restrictions."
					tags={["git", "curl", "python", "npm", "node"]}
					delay={0}
				/>
				<FeatureCard
					icon={Clock}
					title="Instant coldstart"
					description="Minimal memory overhead. No waiting for VMs to boot. Agents wake instantly when needed."
					metric={{ value: "~5ms", label: "coldstart" }}
					delay={0.05}
				/>
				<FeatureCard
					icon={FolderOpen}
					title="Real file system"
					description="Not a mock. A real, persistent file system agents can read, write, and navigate like any Linux environment."
					delay={0.1}
				/>
				<FeatureCard
					icon={Shield}
					title="Granular security"
					description="V8 isolates + WebAssembly. Hardware-level isolation without the overhead. Control exactly what agents can access."
					delay={0.15}
				/>
				<FeatureCard
					icon={Layers}
					title="Hybrid execution"
					description="Lightweight isolation by default. Spin up full sandboxes when you need them. Best of both worlds."
					delay={0.2}
				/>
				<FeatureCard
					icon={Globe}
					title="Runs anywhere"
					description="Railway, Kubernetes, browsers, edge. No microVMs. No special infrastructure. Just npm install and go."
					tags={["Railway", "K8s", "Browsers", "Edge"]}
					delay={0.25}
				/>
			</div>

			{/* Philosophy quote */}
			<motion.div
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className="mx-auto max-w-3xl"
			>
				<blockquote className="border-l-4 border-white pl-6 text-lg italic leading-relaxed text-zinc-400 md:text-xl">
					"Unix was built on the insight that humans think in files and text
					streams. AgentOS is built on the insight that agents need Linux-like
					power without Linux-like overhead."
				</blockquote>
			</motion.div>
		</div>
	</section>
);

// --- Design Principles ---
const DesignPrinciples = () => {
	const principles = [
		{
			number: "01",
			title: "State is primary",
			description:
				"Agents need memory. Not just databases. Embedded state that moves with them, survives restarts, and replicates across regions.",
		},
		{
			number: "02",
			title: "Security by default",
			description:
				"Agents run untrusted code. The security boundary isn't optional. It's the foundation. Isolation without overhead.",
		},
		{
			number: "03",
			title: "Agent-agnostic",
			description:
				"Claude Code today, Codex tomorrow, something new next month. The operating system shouldn't care which agent you run.",
		},
		{
			number: "04",
			title: "Real-time native",
			description:
				"Agents communicate constantly. WebSockets aren't an add-on. They're built into the runtime. Events flow without polling.",
		},
		{
			number: "05",
			title: "Scale to zero",
			description:
				"Millions of agents, most idle. Pay for what runs. Wake in milliseconds when needed. No wasted capacity.",
		},
		{
			number: "06",
			title: "Deploy anywhere",
			description:
				"Your cloud, our cloud, bare metal. The agent OS shouldn't lock you into a platform. Open source, self-hostable, portable.",
		},
	];

	return (
		<section
			id="principles"
			className="border-t border-white/10 py-24 md:py-32"
		>
			<div className="mx-auto max-w-7xl px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-16 text-center text-3xl font-extrabold tracking-tight text-white md:text-4xl"
				>
					Design principles for the agent era
				</motion.h2>

				<div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
					{principles.map((p, i) => (
						<motion.div
							key={p.number}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: i * 0.06 }}
							className="rounded-lg border border-white/10 bg-white/[0.02] p-6 transition-colors hover:border-white/20"
						>
							<span className="mb-4 block font-mono text-xs font-semibold text-zinc-600">
								{p.number}
							</span>
							<h3 className="mb-3 text-lg font-bold tracking-tight text-white">
								{p.title}
							</h3>
							<p className="text-sm leading-relaxed text-zinc-400">
								{p.description}
							</p>
						</motion.div>
					))}
				</div>
			</div>
		</section>
	);
};

// --- CTA ---
const CTASection = () => (
	<section className="border-t border-white/10 py-24 md:py-32">
		<div className="mx-auto max-w-3xl px-6 text-center">
			<motion.h2
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className="mb-4 text-3xl font-extrabold tracking-tight text-white md:text-4xl"
			>
				Build for the agent era
			</motion.h2>
			<motion.p
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5, delay: 0.1 }}
				className="mx-auto mb-8 max-w-lg text-base leading-relaxed text-zinc-500"
			>
				The next generation of software will be built by agents, for agents.
				Start building today.
			</motion.p>
			<motion.div
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5, delay: 0.2 }}
				className="flex flex-col items-center justify-center gap-3 sm:flex-row"
			>
				<a
					href="https://dashboard.rivet.dev"
					className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-5 py-2.5 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
				>
					Get Started Free
					<ArrowRight className="h-4 w-4" />
				</a>
				<a
					href="/docs"
					className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-5 py-2.5 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
				>
					Read the Docs
				</a>
			</motion.div>
		</div>
	</section>
);

// --- Main Page ---
export default function AgentOSPage() {
	return (
		<div className="min-h-screen bg-black font-sans text-zinc-300 selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<Hero />
				<Timeline />
				<ShiftSection />
				<AgentOSFeatures />
				<DesignPrinciples />
				<CTASection />
			</main>
		</div>
	);
}

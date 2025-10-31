import Image from "next/image";
import imgStudio from "@/images/screenshots/rivet-hub.png";
import { Icon } from "@rivet-gg/icons";
import {
	faTerminal,
	faChartLine,
	faBug,
	faHeartPulse,
	faUserGroup,
	faGaugeHigh,
	faCodeBranch,
	faLeaf,
	faEye,
	faFlask,
	faMagnifyingGlass,
	faNetworkWired,
} from "@rivet-gg/icons";

export interface FeatureItem {
	icon: any;
	title?: string;
	name?: string;
	description: string;
}

interface FeatureProps {
	feature: FeatureItem;
}

function Feature({ feature }: FeatureProps) {
	const title = feature.title || feature.name;

	return (
		<div className="p-5 text-md">
			<div className="flex items-center gap-3 mb-3 text-white/90">
				<Icon icon={feature.icon} className="w-5 h-5 text-white" />
				<span className="font-medium text-white">{title}</span>
			</div>
			<p className="text-white/60">{feature.description}</p>
		</div>
	);
}

export function StudioSection() {
	const features: FeatureItem[] = [
		{
			name: "Live State Inspection",
			icon: faEye,
			description:
				"View and edit your actor state in real-time as messages are sent and processed",
		},
		{
			name: "Event Monitoring",
			icon: faChartLine,
			description:
				"See all events happening in your actor in real-time - track every state change and action as it happens",
		},
		{
			name: "REPL",
			icon: faTerminal,
			description:
				"Debug your actor in real-time - call actions, subscribe to events, and interact directly with your code",
		},
		{
			name: "Connection Inspection",
			icon: faNetworkWired,
			description:
				"Monitor active connections with state and parameters for each client",
		},
	];

	return (
		<div className="w-full px-6">
			<div className="relative group">
				{/* Unified hover area covering screenshot and spacer */}
				{/*href="https://www.youtube.com/watch?v=RYgo25fH9Ss"*/}
				<a
					className="absolute cursor-pointer"
					style={{
						top: "200px",
						left: "0",
						right: "0",
						height: "400px",
						width: "100%",
						zIndex: 15,
					}}
					href="https://x.com/NathanFlurry/status/1976427064678023634"
					target="_blank"
					rel="noopener noreferrer"
				/>

				{/* Content */}
				<div className="pointer-events-none">
					{/* Header */}
					<div className="max-w-7xl mx-auto relative z-20 pointer-events-auto">
						<h2 className="max-w-lg text-4xl font-medium tracking-tight text-white">
							Built-In Observability
						</h2>
						<p className="max-w-lg mt-4 text-lg text-white/70">
							Powerful debugging and monitoring tools that work from local development to production.
						</p>

						{/* Visit the Studio link */}
						<div className="mt-3">
							<a
								href="https://inspect.rivet.dev"
								className="inline-flex items-center gap-2 text-[#FF5C00] hover:text-[#FF5C00]/80 transition-colors group/link"
								target="_blank"
								rel="noopener noreferrer"
							>
								Visit The Inspector
								<span className="transition-transform group-hover/link:translate-x-1">
									→
								</span>
							</a>
						</div>
					</div>

					{/* Spacer with Watch Demo button */}
					<div className="h-[380px] relative flex items-center justify-center">
						{/* Watch Demo overlay that appears on hover */}
						<div className="absolute opacity-0 group-hover:opacity-100 transition-opacity duration-300 z-20 pointer-events-none">
							<div className="bg-white/20 backdrop-blur-sm border border-white/30 rounded-lg px-6 py-3 text-white font-medium flex items-center gap-2">
								<svg
									className="w-5 h-5"
									fill="currentColor"
									viewBox="0 0 20 20"
								>
									<path d="M6.3 2.84A1 1 0 004 3.75v12.5a1 1 0 001.59.81l11-6.25a1 1 0 000-1.62l-11-6.25a1 1 0 00-1.29.06z" />
								</svg>
								Watch Demo
							</div>
						</div>
					</div>

					{/* Features grid */}
					<div className="pt-16 relative bg-gradient-to-t from-[hsl(var(--background))] via-[hsl(var(--background))] to-transparent z-20 pointer-events-auto">
						<div className="max-w-7xl mx-auto grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6 gap-x-6 gap-y-6">
							{features.map((feature, index) => (
								<Feature key={index} feature={feature} />
							))}
						</div>
					</div>
				</div>

				{/* Screenshot */}
				<div className="absolute inset-0 overflow-hidden">
					{/* Screenshot wrapper */}
					<div
						className="absolute"
						style={{
							top: "220px",
							left: "0",
							right: "0",
							width: "80%",
							maxWidth: "1200px",
							margin: "0 auto",
							aspectRatio: "16/9",
							zIndex: 10,
						}}
					>
						{/* Perspective container that gets blurred and dimmed */}
						<div
							className="w-full h-full transition-all duration-300 group-hover:blur-sm group-hover:brightness-75"
							style={{ perspective: "2000px" }}
						>
							<div
								className="border-2 border-white/10 rounded-md w-full h-full"
								style={{
									transformStyle: "preserve-3d",
									transform:
										"translateX(-11%) scale(1.2) rotateX(38deg) rotateY(19deg) rotateZ(340deg)",
									transformOrigin: "top left",
									boxShadow:
										"0 35px 60px -15px rgba(0, 0, 0, 0.5)",
								}}
							>
								{/* Studio screenshot with enhanced depth */}
								<div
									className="w-full h-full overflow-hidden relative rounded-md"
									style={{
										transformStyle: "preserve-3d",
										backfaceVisibility: "hidden",
									}}
								>
									<Image
										src={imgStudio}
										alt="Rivet Dashboard"
										className="w-full h-full object-cover object-top rounded-md"
										fill
										sizes="(max-width: 768px) 100vw, 80vw"
										priority
										quality={90}
									/>
								</div>
							</div>
						</div>
					</div>

					{/* Gradient overlay */}
					<div
						className="absolute inset-0 z-[1] pointer-events-none"
						style={{
							background:
								"linear-gradient(90deg, hsl(var(--background) / 0) 66%, hsl(var(--background) / 0.95) 85%, hsl(var(--background) / 1) 100%)",
						}}
					/>
				</div>
			</div>
		</div>
	);
}

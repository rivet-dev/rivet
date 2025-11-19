"use client";

import { Zap, Database, Cpu, AlertCircle } from "lucide-react";
import { motion } from "framer-motion";

const FeatureCard = ({ title, description, code, graphic }) => (
	<div className="group relative overflow-hidden rounded-3xl border border-white/10 bg-white/[0.02] hover:bg-white/[0.04] backdrop-blur-sm transition-all duration-500 hover:border-white/20 flex flex-col h-full hover:shadow-[0_0_50px_-12px_rgba(255,255,255,0.1)]">
		{/* Graphic Area */}
		<div className="h-48 bg-white/[0.01] border-b border-white/5 flex items-center justify-center relative overflow-hidden group-hover:bg-white/[0.02] transition-colors">
			{graphic}
		</div>

		{/* Content */}
		<div className="p-8 flex flex-col flex-grow">
			<h3 className="text-xl font-medium text-white mb-3 tracking-tight">{title}</h3>
			<p className="text-sm text-zinc-400 leading-relaxed mb-6 flex-grow">{description}</p>

			{code && (
				<div className="self-start px-3 py-1.5 rounded bg-black/50 border border-white/5 text-xs font-mono text-[#FF4500]">
					{code}
				</div>
			)}
		</div>
	</div>
);

export const FeaturesSection = () => {
	const features = [
		{
			title: "Long-Lived Compute",
			description:
				"Like Lambda, but with memory. No 5-minute timeouts, no state loss. Your logic runs as long as your product does.",
			code: "persistent.process()",
			graphic: (
				<div className="flex flex-col justify-center gap-6 w-full px-12 h-full">
					{/* Stateless Side */}
					<div className="flex items-center gap-4 opacity-50 group-hover:opacity-80 transition-opacity">
						<div className="w-20 text-[10px] text-zinc-500 font-mono text-right tracking-wide">
							stateless
						</div>
						<div className="flex-1 h-[2px] bg-zinc-800 relative overflow-hidden rounded-full">
							<div className="absolute inset-0 bg-zinc-400 w-full origin-left animate-[statelessChurn_3s_ease-in-out_infinite]" />
						</div>
					</div>

					{/* Stateful Side */}
					<div className="flex items-center gap-4">
						<div className="w-20 text-[10px] text-[#FF4500] font-bold font-mono text-right tracking-wide">
							stateful
						</div>
						<div className="flex-1 h-[2px] bg-zinc-800 relative rounded-full overflow-hidden">
							<div className="absolute inset-0 bg-[#FF4500] w-full origin-left animate-[statefulLifecycle_6s_ease-out_infinite]" />
							<div className="absolute inset-0 bg-white/30 w-full -translate-x-full animate-[shimmer_2s_infinite]" />
						</div>
					</div>
				</div>
			),
		},
		{
			title: "In-Memory Speed",
			description:
				"State lives beside your compute. Reads and writes are in-memory—no cache invalidation, no round-trips.",
			code: "read(<1ms)",
			graphic: (
				<div className="relative w-full h-full flex items-center justify-center">
					<div className="flex items-center gap-0">
						{/* The Actor Box */}
						<div className="relative w-32 h-24 border border-[#FF4500]/30 bg-[#FF4500]/10 rounded-xl p-2 flex flex-col justify-between z-10 backdrop-blur-sm">
							<div className="text-[10px] text-[#FF4500] font-mono uppercase tracking-wider mb-1 text-center">
								Actor
							</div>
							<div className="flex items-center justify-around flex-1 relative px-1">
								{/* Internal Pipe */}
								<div className="absolute top-1/2 left-8 right-8 h-1 bg-[#FF4500]/50 rounded-full overflow-hidden -translate-y-1/2 -mt-2">
									<div className="absolute inset-0 w-full h-full bg-gradient-to-r from-transparent via-[#FF4500] to-transparent opacity-90 animate-[shuttle_1.5s_ease-in-out_infinite]" />
								</div>

								{/* CPU Node */}
								<div className="relative z-10 flex flex-col items-center gap-1">
									<div className="w-8 h-8 bg-zinc-900 rounded border border-[#FF4500]/50 flex items-center justify-center shadow-[0_0_15px_rgba(255,69,0,0.1)]">
										<Cpu className="w-4 h-4 text-[#FF4500]" />
									</div>
									<span className="text-[8px] text-[#FF4500]/70">Compute</span>
								</div>

								{/* Local State Node */}
								<div className="relative z-10 flex flex-col items-center gap-1">
									<div className="w-8 h-8 bg-zinc-900 rounded border border-[#FF4500]/50 flex items-center justify-center shadow-[0_0_15px_rgba(255,69,0,0.1)]">
										<Database className="w-4 h-4 text-[#FF4500]" />
									</div>
									<span className="text-[8px] text-[#FF4500]/70">In-Mem</span>
								</div>
							</div>
						</div>

						{/* External Pipe */}
						<div className="w-24 h-1 bg-zinc-800 rounded-full relative overflow-hidden">
							<div className="absolute inset-0 w-1/2 h-full bg-gradient-to-r from-transparent via-zinc-500 to-transparent animate-[shuttle_4s_ease-in-out_infinite]" />
						</div>

						{/* External DB */}
						<div className="flex flex-col items-center gap-1">
							<div className="w-10 h-10 bg-zinc-900 rounded-full border border-zinc-700 flex items-center justify-center z-0 shadow-lg">
								<Database className="w-4 h-4 text-zinc-500" />
							</div>
							<span className="text-[8px] text-zinc-600">DB</span>
						</div>
					</div>
				</div>
			),
		},
		{
			title: "Realtime, Built-in",
			description:
				"WebSockets and SSE out of the box. Broadcast updates with one line—no extra infrastructure, no pub/sub layer.",
			code: "c.broadcast()",
			graphic: (
				<div className="relative flex items-center justify-center w-full h-full overflow-hidden">
					{/* Center Node */}
					<div className="relative z-10 w-4 h-4 bg-white rounded-full shadow-[0_0_20px_white]" />

					{/* Ripples */}
					<div className="absolute w-16 h-16 border border-[#FF4500]/30 rounded-full animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite]" />
					<div className="absolute w-32 h-32 border border-[#FF4500]/20 rounded-full animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite_0.5s]" />
					<div className="absolute w-48 h-48 border border-[#FF4500]/10 rounded-full animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite_1s]" />

					{/* Satellite Nodes */}
					<div className="absolute top-10 left-1/4 w-2 h-2 bg-[#FF4500] rounded-full" />
					<div className="absolute bottom-10 right-1/4 w-2 h-2 bg-[#FF4500] rounded-full" />
					<div className="absolute top-1/2 right-10 w-2 h-2 bg-[#FF4500] rounded-full" />
				</div>
			),
		},
		{
			title: "Automatic Hibernation",
			description:
				"Actors automatically hibernate to save costs and wake instantly on demand. You only pay for work done.",
			code: "idle → sleep()",
			graphic: (
				<div className="relative w-full h-full flex flex-col items-center justify-center">
					{/* Packet moves in from left */}
					<div className="absolute left-[20%] top-1/2 -translate-y-1/2 w-3 h-3 bg-white rounded-full shadow-[0_0_10px_white] animate-[simplePacket_4s_ease-in-out_infinite]" />

					{/* Actor Container */}
					<div className="relative z-10 flex flex-col items-center gap-3">
						<div className="w-20 h-20 rounded-2xl border flex items-center justify-center shadow-2xl bg-zinc-900 relative overflow-hidden animate-[boxState_4s_ease-in-out_infinite]">
							{/* Awake State Content (Zap) */}
							<div className="absolute inset-0 grid place-items-center animate-[fadeZap_4s_ease-in-out_infinite]">
								<Zap className="w-10 h-10 text-yellow-400 fill-yellow-400/20 drop-shadow-[0_0_15px_rgba(250,204,21,0.6)]" />
							</div>

							{/* Sleep State Content (Zzz) */}
							<div className="absolute inset-0 grid place-items-center animate-[fadeZzz_4s_ease-in-out_infinite]">
								<div className="flex items-end gap-[1px] mb-1">
									<span
										className="text-2xl font-bold text-zinc-600 animate-[float_3s_ease-in-out_infinite]"
										style={{ animationDelay: "0s" }}
									>
										Z
									</span>
									<span
										className="text-xl font-bold text-zinc-700 animate-[float_3s_ease-in-out_infinite]"
										style={{ animationDelay: "0.5s" }}
									>
										z
									</span>
									<span
										className="text-sm font-bold text-zinc-800 animate-[float_3s_ease-in-out_infinite]"
										style={{ animationDelay: "1s" }}
									>
										z
									</span>
								</div>
							</div>
						</div>
					</div>
				</div>
			),
		},
		{
			title: "Open Source",
			description:
				"No lock-in. Run on your platform of choice or bare metal with the same API and mental model.",
			code: "apache-2.0",
			graphic: (
				<div className="w-48 h-32 bg-zinc-950 rounded-lg border border-white/10 p-3 font-mono text-[10px] text-zinc-500 flex flex-col gap-1.5 shadow-2xl transform rotate-3 hover:rotate-0 transition-transform duration-500">
					<div className="flex gap-1.5 mb-1 opacity-50">
						<div className="w-2 h-2 rounded-full bg-white" />
						<div className="w-2 h-2 rounded-full bg-white" />
						<div className="w-2 h-2 rounded-full bg-white" />
					</div>
					<div className="text-[#FF4500] flex gap-2">
						<span className="select-none">$</span> cargo run
					</div>
					<div>Compiling rivet...</div>
					<div className="text-zinc-300">Finished dev profile</div>
					<div className="flex gap-1 items-center text-[#FF4500] mt-1">
						<span>&gt;</span>
						<span className="w-1.5 h-3 bg-[#FF4500] animate-pulse" />
					</div>
				</div>
			),
		},
		{
			title: "Resilient by Design",
			description:
				"Automatic failover and restarts maintain state integrity. Your actors survive crashes, deploys, noisy neighbors.",
			code: "uptime: 99.9%",
			graphic: (
				<div className="relative w-full h-full flex items-center justify-center">
					{/* Pulse Ring Background */}
					<div className="absolute w-32 h-32 bg-[#FF4500]/5 rounded-full animate-[ping_3s_cubic-bezier(0,0,0.2,1)_infinite]" />

					{/* The Process Shell */}
					<div className="relative z-10 w-16 h-16 rounded-xl border-2 flex items-center justify-center transition-colors duration-300 animate-[shellRecover_4s_ease-in-out_infinite] bg-zinc-900">
						{/* The Persistent State (Core) */}
						<div className="text-white z-20">
							<Database className="w-6 h-6 text-[#FF4500] fill-[#FF4500]/20 drop-shadow-[0_0_10px_rgba(255,69,0,0.5)]" />
						</div>

						{/* Crash indicator overlay */}
						<div className="absolute -top-3 -right-3 flex items-center justify-center text-red-500 opacity-0 animate-[crashIcon_4s_ease-in-out_infinite] z-30 bg-zinc-900 rounded-full border border-red-500/50 shadow-lg shadow-red-500/20 p-1">
							<AlertCircle className="w-5 h-5 fill-red-500/20" />
						</div>
					</div>

					{/* "Rebooting" Spinner ring */}
					<div className="absolute w-24 h-24 rounded-full border-2 border-[#FF4500]/50 border-t-[#FF4500] opacity-0 animate-[spinRecover_4s_ease-in-out_infinite]" />
				</div>
			),
		},
	];

	return (
		<section id="features" className="py-32 bg-black relative">
			<div className="max-w-7xl mx-auto px-6">
				<div className="mb-20 max-w-2xl">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Everything you need for
						<br />
						stateful workloads.
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400"
					>
						Rivet handles the hard parts of distributed systems: sharding, coordination, and persistence.
						You just write the logic.
					</motion.p>
				</div>

				<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
					{features.map((feature, idx) => (
						<motion.div
							key={idx}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: idx * 0.1 }}
						>
							<FeatureCard {...feature} />
						</motion.div>
					))}
				</div>
			</div>

			{/* Custom Animations for Graphics */}
			<style>{`
        @keyframes statelessChurn {
          0% { transform: scaleX(0); background-color: rgb(161 161 170); }
          20% { transform: scaleX(1); background-color: rgb(161 161 170); }
          50% { transform: scaleX(1); background-color: rgb(161 161 170); opacity: 1; }
          55% { transform: scaleX(1); background-color: rgb(239 68 68); opacity: 1; }
          60% { transform: scaleX(1); opacity: 0; }
          100% { transform: scaleX(1); opacity: 0; }
        }

        @keyframes statefulLifecycle {
          0% { transform: scaleX(0); opacity: 1; }
          10% { transform: scaleX(1); opacity: 1; }
          30% { opacity: 1; }
          40% { opacity: 0.3; }
          80% { opacity: 0.3; }
          90% { opacity: 1; }
          100% { opacity: 1; }
        }

        @keyframes shimmer {
          0% { transform: translateX(-100%); }
          100% { transform: translateX(100%); }
        }

        @keyframes shuttle {
          0% { transform: translateX(-100%); }
          50% { transform: translateX(100%); }
          100% { transform: translateX(-100%); }
        }

        @keyframes simplePacket {
          0% { left: 15%; opacity: 0; transform: translate(-50%, -50%) scale(0.5); }
          10% { opacity: 1; transform: translate(-50%, -50%) scale(1); }
          40% { left: 50%; opacity: 1; transform: translate(-50%, -50%) scale(1); }
          42% { left: 50%; opacity: 0; transform: translate(-50%, -50%) scale(1.2); }
          100% { left: 50%; opacity: 0; }
        }

        @keyframes boxState {
          0%, 39% { border-color: rgb(39 39 42); box-shadow: none; }
          40%, 90% { border-color: rgb(255, 69, 0); box-shadow: 0 0 20px -5px rgba(255, 69, 0, 0.3); }
          95%, 100% { border-color: rgb(39 39 42); box-shadow: none; }
        }

        @keyframes fadeZap {
          0%, 39% { opacity: 0; transform: scale(0.9); }
          42%, 90% { opacity: 1; transform: scale(1); }
          95%, 100% { opacity: 0; transform: scale(0.9); }
        }

        @keyframes fadeZzz {
          0%, 39% { opacity: 1; transform: scale(1); }
          42%, 90% { opacity: 0; transform: scale(0.9); }
          95%, 100% { opacity: 1; transform: scale(1); }
        }

        @keyframes float {
          0%, 100% { transform: translateY(0); }
          50% { transform: translateY(-4px); }
        }

        @keyframes shellRecover {
          0%, 45% { border-color: rgb(255, 69, 0); }
          50% { border-color: rgb(239 68 68); border-style: dashed; }
          60% { border-color: transparent; }
          70%, 100% { border-color: rgb(255, 69, 0); border-style: solid; }
        }

        @keyframes crashIcon {
          0%, 48% { opacity: 0; transform: scale(0.5); }
          50%, 55% { opacity: 1; transform: scale(1.2); }
          60%, 100% { opacity: 0; transform: scale(0.5); }
        }

        @keyframes spinRecover {
          0%, 55% { opacity: 0; transform: rotate(0deg); }
          60% { opacity: 1; }
          70% { opacity: 1; transform: rotate(360deg); }
          80%, 100% { opacity: 0; transform: rotate(360deg); }
        }
      `}</style>
		</section>
	);
};

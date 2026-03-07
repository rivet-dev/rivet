'use client';

import { motion } from 'framer-motion';
import { Sparkles, Bot } from 'lucide-react';

const cards = [
	{
		icon: Sparkles,
		title: 'AI-generated actors',
		description: 'AI-generated workflows, per-user databases, and custom logic — each running as its own actor.',
		diagram: (
			<svg viewBox="0 0 280 120" fill="none" className='h-auto w-full'>
				{/* AI box */}
				<rect x="10" y="35" width="70" height="50" rx="8" stroke="white" strokeOpacity={0.15} strokeWidth={1} />
				<text x="45" y="64" textAnchor="middle" fill="white" fillOpacity={0.5} fontSize="11" fontFamily="monospace">AI</text>
				{/* Arrow */}
				<path d="M 85 60 L 125 60" stroke="white" strokeOpacity={0.15} strokeWidth={1} />
				<polygon points="122,56 130,60 122,64" fill="white" fillOpacity={0.2} />
				{/* Generated actors */}
				<rect x="135" y="10" width="60" height="35" rx="6" stroke="#FF4500" strokeOpacity={0.3} strokeWidth={1} fill="#FF4500" fillOpacity={0.05} />
				<text x="165" y="31" textAnchor="middle" fill="white" fillOpacity={0.5} fontSize="9" fontFamily="monospace">actor</text>
				<rect x="135" y="52" width="60" height="35" rx="6" stroke="#FF4500" strokeOpacity={0.3} strokeWidth={1} fill="#FF4500" fillOpacity={0.05} />
				<text x="165" y="73" textAnchor="middle" fill="white" fillOpacity={0.5} fontSize="9" fontFamily="monospace">actor</text>
				<rect x="135" y="94" width="60" height="35" rx="6" stroke="#FF4500" strokeOpacity={0.3} strokeWidth={1} fill="#FF4500" fillOpacity={0.05} />
				<text x="165" y="115" textAnchor="middle" fill="white" fillOpacity={0.5} fontSize="9" fontFamily="monospace">actor</text>
				{/* Branch arrows */}
				<path d="M 130 60 L 135 27" stroke="white" strokeOpacity={0.1} strokeWidth={1} />
				<path d="M 130 60 L 135 69" stroke="white" strokeOpacity={0.1} strokeWidth={1} />
				<path d="M 130 60 L 135 111" stroke="white" strokeOpacity={0.1} strokeWidth={1} />
			</svg>
		),
	},
	{
		icon: Bot,
		title: 'Automate coding agents',
		description: 'Orchestrate coding agents in sandboxes, each with their own persistent state and tools.',
		diagram: (
			<svg viewBox="0 0 280 120" fill="none" className='h-auto w-full'>
				{/* Orchestrator */}
				<rect x="90" y="5" width="100" height="35" rx="8" stroke="white" strokeOpacity={0.15} strokeWidth={1} />
				<text x="140" y="26" textAnchor="middle" fill="white" fillOpacity={0.5} fontSize="10" fontFamily="monospace">orchestrator</text>
				{/* Arrows down */}
				<path d="M 115 40 L 55 75" stroke="white" strokeOpacity={0.15} strokeWidth={1} />
				<path d="M 140 40 L 140 75" stroke="white" strokeOpacity={0.15} strokeWidth={1} />
				<path d="M 165 40 L 225 75" stroke="white" strokeOpacity={0.15} strokeWidth={1} />
				{/* Agent sandboxes */}
				<rect x="15" y="75" width="80" height="40" rx="6" stroke="#FF4500" strokeOpacity={0.3} strokeWidth={1} fill="#FF4500" fillOpacity={0.05} />
				<text x="55" y="99" textAnchor="middle" fill="white" fillOpacity={0.5} fontSize="9" fontFamily="monospace">sandbox</text>
				<rect x="100" y="75" width="80" height="40" rx="6" stroke="#FF4500" strokeOpacity={0.3} strokeWidth={1} fill="#FF4500" fillOpacity={0.05} />
				<text x="140" y="99" textAnchor="middle" fill="white" fillOpacity={0.5} fontSize="9" fontFamily="monospace">sandbox</text>
				<rect x="185" y="75" width="80" height="40" rx="6" stroke="#FF4500" strokeOpacity={0.3} strokeWidth={1} fill="#FF4500" fillOpacity={0.05} />
				<text x="225" y="99" textAnchor="middle" fill="white" fillOpacity={0.5} fontSize="9" fontFamily="monospace">sandbox</text>
			</svg>
		),
	},
];

export const AIAppsSection = () => {
	return (
		<section className='border-t border-white/10 px-6 py-20 lg:py-32'>
			<div className='mx-auto max-w-7xl'>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className='mb-12'
				>
					<h2 className='mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl'>
						For AI-generated apps
					</h2>
					<p className='text-base leading-relaxed text-zinc-500'>
						Let AI build and orchestrate backend logic as actors.
					</p>
				</motion.div>

				<div className='grid grid-cols-1 gap-6 md:grid-cols-2'>
					{cards.map((card, idx) => {
						const Icon = card.icon;
						return (
							<motion.div
								key={card.title}
								initial={{ opacity: 0, y: 20 }}
								whileInView={{ opacity: 1, y: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.4, delay: idx * 0.05 }}
								className='group rounded-xl border border-white/10 bg-white/[0.03] p-6 transition-colors duration-200 hover:border-white/20'
							>
								<div className='mb-4 flex items-center gap-3'>
									<Icon className='h-5 w-5 text-zinc-400' />
									<h3 className='text-base font-medium text-white'>{card.title}</h3>
								</div>
								<p className='mb-6 text-sm leading-relaxed text-zinc-500'>
									{card.description}
								</p>
								<div className='rounded-lg border border-white/[0.06] bg-white/[0.02] p-4'>
									{card.diagram}
								</div>
							</motion.div>
						);
					})}
				</div>
			</div>
		</section>
	);
};

'use client';

import { GitBranch, Database, Bot } from 'lucide-react';
import { motion } from 'framer-motion';

const useCases = [
	{
		icon: GitBranch,
		title: 'AI-Generated Workflows',
		description: 'Your agents write multi-step workflows that run as durable actors.',
	},
	{
		icon: Database,
		title: 'AI-Generated Databases',
		description: 'Spin up per-tenant databases from AI-authored schemas.',
	},
	{
		icon: Bot,
		title: 'AI-Generated Agents',
		description: 'Deploy autonomous agents from generated code with full actor capabilities.',
	},
];

export const DynamicActorsSection = () => {
	return (
		<section className='border-t border-white/10 px-6 py-16 lg:py-24'>
			<div className='mx-auto w-full max-w-7xl'>
				<div className='grid grid-cols-1 gap-12 lg:grid-cols-2'>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
					>
						<h2 className='mb-4 text-2xl font-normal tracking-tight text-white md:text-4xl'>
							Securely Run AI-Generated Code
						</h2>
						<p className='max-w-lg text-base leading-relaxed text-zinc-500'>
							Dynamic Actors let your AI generate and deploy actors at runtime. Each actor runs in its own isolated process with built-in state, storage, and networking — no containers, no cold infrastructure.
						</p>
					</motion.div>

					<div className='flex flex-col gap-6'>
						{useCases.map((useCase, idx) => {
							const Icon = useCase.icon;
							return (
								<motion.div
									key={useCase.title}
									initial={{ opacity: 0, x: 20 }}
									whileInView={{ opacity: 1, x: 0 }}
									viewport={{ once: true }}
									transition={{ duration: 0.4, delay: idx * 0.05 }}
									className='flex gap-4 border-l border-white/10 pl-6'
								>
									<Icon className='mt-0.5 h-5 w-5 flex-shrink-0 text-zinc-500' />
									<div>
										<h3 className='text-sm font-medium text-white'>{useCase.title}</h3>
										<p className='mt-1 text-xs leading-relaxed text-zinc-500'>{useCase.description}</p>
									</div>
								</motion.div>
							);
						})}
					</div>
				</div>
			</div>
		</section>
	);
};

'use client';

import { useState } from 'react';
import { motion } from 'framer-motion';
import { Check, Copy, ArrowRight } from 'lucide-react';

const CopyCommand = ({ command }: { command: string }) => {
	const [copied, setCopied] = useState(false);

	const handleCopy = async () => {
		await navigator.clipboard.writeText(command);
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	};

	return (
		<div className='group relative flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.03] px-6 py-4 font-mono text-sm'>
			<span className='text-zinc-500'>$</span>
			<code className='flex-1 text-zinc-200'>{command}</code>
			<button
				onClick={handleCopy}
				className='flex h-8 w-8 items-center justify-center rounded-md border border-white/10 text-zinc-400 transition-colors hover:border-white/25 hover:text-white'
			>
				{copied ? <Check className='h-4 w-4 text-emerald-500' /> : <Copy className='h-4 w-4' />}
			</button>
		</div>
	);
};

export default function GetStartedPage() {
	return (
		<div className='flex min-h-screen flex-col items-center justify-center overflow-x-hidden bg-black selection:bg-[#FF4500]/30 selection:text-orange-200'>
			{/* Hero */}
			<section className='px-6'>
				<div className='mx-auto max-w-3xl text-center'>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className='mb-10 flex items-center justify-center'
					>
						<img
							src='/images/agent-os/agentos-hero-logo.svg'
							alt='agentOS'
							className='h-16 w-auto invert md:h-20'
						/>
					</motion.div>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className='mx-auto max-w-xl flex flex-col gap-4'
					>
						<CopyCommand command='npm install rivetkit' />
						<a
							href='/docs/agent-os/quickstart'
							className='selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200'
						>
							Quickstart Guide
							<ArrowRight className='h-4 w-4' />
						</a>
					</motion.div>
				</div>
			</section>
		</div>
	);
}

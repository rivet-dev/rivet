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
		<div className='group relative flex items-center gap-3 rounded-xl border border-zinc-200 bg-zinc-50 px-6 py-4 font-mono text-sm'>
			<span className='text-zinc-400'>$</span>
			<code className='flex-1 text-zinc-900'>{command}</code>
			<button
				onClick={handleCopy}
				className='flex h-8 w-8 items-center justify-center rounded-md border border-zinc-200 bg-white text-zinc-500 transition-colors hover:border-zinc-300 hover:text-zinc-900'
			>
				{copied ? <Check className='h-4 w-4 text-emerald-500' /> : <Copy className='h-4 w-4' />}
			</button>
		</div>
	);
};

export default function GetStartedPage() {
	return (
		<div className='flex min-h-screen flex-col items-center justify-center bg-white selection:bg-zinc-200 selection:text-zinc-900'>
			{/* Hero */}
			<section className='px-6'>
				<div className='mx-auto max-w-3xl text-center'>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className='mb-10 flex items-center justify-center'
					>
						<div className='relative'>
							<img
								src='/images/agent-os/agentos-hero-logo.svg'
								alt='AgentOS'
								className='h-16 w-auto md:h-20'
							/>
							<span className='absolute -top-2 -right-12 rounded-full bg-emerald-100 px-2 py-0.5 text-[10px] font-medium text-emerald-700'>Beta</span>
						</div>
					</motion.div>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className='mx-auto max-w-xl flex flex-col gap-4'
					>
						<CopyCommand command='npm install @rivetkit/agent-os' />
						<a
							href='/docs/actors'
							className='inline-flex items-center justify-center gap-3 rounded-md bg-zinc-900 px-6 py-3 text-sm font-medium text-white transition-colors hover:bg-zinc-700'
						>
							<span className='text-zinc-400'>{'>_'}</span>
							Quickstart Guide
							<ArrowRight className='h-4 w-4' />
						</a>
					</motion.div>
				</div>
			</section>
		</div>
	);
}

'use client';

import { useState } from 'react';
import { motion } from 'framer-motion';
import { Check, Copy, ArrowRight } from 'lucide-react';

// The single ink moment on this page: an InkChip-style command strip with a
// copy affordance. Kept local because the shared InkChip has no copy button.
const CopyCommand = ({ command }: { command: string }) => {
	const [copied, setCopied] = useState(false);

	const handleCopy = async () => {
		await navigator.clipboard.writeText(command);
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	};

	return (
		<div className='selection-paper group relative flex items-center gap-3 rounded-lg border border-ink/20 bg-ink px-6 py-4 font-mono text-sm text-cream/85'>
			<span aria-hidden='true' className='select-none text-sage'>
				$
			</span>
			<code className='flex-1 text-left'>{command}</code>
			<button
				onClick={handleCopy}
				aria-label='Copy install command'
				className='flex h-8 w-8 items-center justify-center rounded-md border border-cream/15 text-cream/60 transition-colors hover:border-cream/35 hover:text-cream'
			>
				{copied ? <Check className='h-4 w-4 text-sage' /> : <Copy className='h-4 w-4' />}
			</button>
		</div>
	);
};

export default function GetStartedPage() {
	return (
		<div className='paper-grain flex min-h-screen flex-col items-center justify-center overflow-x-hidden font-sans text-ink-soft'>
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
							className='h-16 w-auto md:h-20'
						/>
					</motion.div>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className='mx-auto flex max-w-xl flex-col gap-4'
					>
						<CopyCommand command='npm install rivetkit' />
						<a
							href='/docs/agent-os/quickstart'
							className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 text-sm text-ink-soft transition-colors hover:border-ink/40 hover:text-ink'
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

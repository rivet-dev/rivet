'use client';

import { motion } from 'framer-motion';

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
		className='grid grid-cols-1 gap-6 md:grid-cols-[100px_1fr] md:gap-12'
	>
		<div className='flex items-start gap-4 md:flex-col md:items-center'>
			<span
				className={`font-mono text-sm font-medium ${future ? 'text-zinc-900' : 'text-zinc-500'}`}
			>
				{year}
			</span>
			<div
				className={`hidden h-full w-px md:block ${future ? 'bg-zinc-900' : 'bg-zinc-200'}`}
			/>
		</div>
		<div className='pb-16'>
			<h2
				className={`mb-4 tracking-tight text-zinc-900 ${future ? 'text-3xl font-normal md:text-4xl' : 'text-2xl font-normal md:text-3xl'}`}
			>
				{title}
			</h2>
			<p className='mb-4 text-base leading-relaxed text-zinc-500 md:text-lg'>
				{lead}
			</p>
			{body && (
				<p className='mb-6 text-sm leading-relaxed text-zinc-500 md:text-base'>
					{body}
				</p>
			)}
			{children}
		</div>
	</motion.div>
);

const PrincipleChip = ({ label, text }: { label: string; text: string }) => (
	<div className='border-t border-zinc-200 pt-4'>
		<span className='mb-2 block font-mono text-[11px] font-medium uppercase tracking-wider text-zinc-500'>
			{label}
		</span>
		<p className='text-sm leading-relaxed text-zinc-500'>{text}</p>
	</div>
);

export default function TimelinePage() {
	return (
		<div className='min-h-screen overflow-x-hidden bg-white font-sans text-zinc-600 selection:bg-zinc-200 selection:text-zinc-900'>
			<main>
				{/* Hero */}
				<section className='relative flex min-h-[60svh] flex-col items-center justify-center px-6 pt-32'>
					<div className='mx-auto w-full max-w-3xl text-center'>
						<motion.h1
							initial={{ opacity: 0, y: 20 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ duration: 0.5 }}
							className='mb-4 text-4xl font-normal leading-[1.1] tracking-tight text-zinc-900 md:text-6xl'
						>
							From Unix to Agents
						</motion.h1>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ duration: 0.5, delay: 0.05 }}
							className='text-lg text-zinc-500 md:text-xl'
						>
							The operating system is being reinvented. Again.
						</motion.p>
					</div>
				</section>

				{/* Timeline */}
				<section className='border-t border-zinc-200 py-16 md:py-24'>
					<div className='mx-auto max-w-7xl px-6'>
						<Era
							year='1969'
							title='The Unix Foundation'
							lead='Before Unix, every computer spoke a different language. Programs written for one machine couldn&apos;t run on another. Computing was fragmented, expensive, and inaccessible.'
							body='Unix changed everything. It introduced a radical idea: a portable operating system with a universal interface. Files, processes, pipes, permissions. Simple primitives that composed into infinite complexity.'
						>
							<div className='mb-6 max-w-2xl overflow-hidden rounded-lg border border-zinc-200'>
								<img
									src='/images/agent-os/ken-thompson-dennis-ritchie-1973.jpg'
									alt='Ken Thompson and Dennis Ritchie, creators of Unix, 1973'
									className='w-full object-cover opacity-90'
									loading='lazy'
								/>
								<p className='bg-zinc-50 px-4 py-2 text-xs text-zinc-600'>
									Ken Thompson and Dennis Ritchie, 1973.{' '}
									<a
										href='https://commons.wikimedia.org/w/index.php?curid=31308'
										className='underline hover:text-zinc-600'
										target='_blank'
										rel='noopener noreferrer'
									>
										Public Domain
									</a>
								</p>
							</div>
							<div className='grid grid-cols-1 gap-3 sm:grid-cols-3'>
								<PrincipleChip
									label='Philosophy'
									text='Do one thing well. Compose small programs into larger systems.'
								/>
								<PrincipleChip
									label='Interface'
									text='Everything is a file. Text streams connect all programs.'
								/>
								<PrincipleChip
									label='Impact'
									text='The foundation for Linux, macOS, Android, and the modern internet.'
								/>
							</div>
						</Era>

						<Era
							year='1991'
							title='Linux & The Open Source Revolution'
							lead='Linux took Unix&apos;s ideas and made them free. Not just free as in cost. Free as in freedom. Anyone could read, modify, and distribute the code that powered their machines.'
							body='This openness sparked an explosion of innovation. The kernel became the backbone of servers, phones, cars, and spacecraft. Open source became the default way to build software.'
							delay={0.1}
						>
							<div className='mt-4 max-w-2xl overflow-hidden rounded-lg border border-zinc-200'>
								<img
									src='/images/agent-os/first-web-server.jpg'
									alt='The first web server at CERN'
									className='w-full object-cover opacity-90'
									loading='lazy'
								/>
								<p className='bg-zinc-50 px-4 py-2 text-xs text-zinc-600'>
									The first web server at CERN. Photo by Coolcaesar,{' '}
									<a
										href='https://commons.wikimedia.org/w/index.php?curid=395096'
										className='underline hover:text-zinc-600'
										target='_blank'
										rel='noopener noreferrer'
									>
										CC BY-SA 3.0
									</a>
								</p>
							</div>
						</Era>

						<Era
							year='2006'
							title='The Cloud Era'
							lead='AWS, then Azure, then GCP. Computing became a utility. No more buying servers. Just rent capacity by the hour. Infrastructure as code. Scale on demand.'
							body='But the fundamental model stayed the same: humans writing code, humans operating systems, humans in the loop at every step. The cloud made computing elastic, but it was still computing for humans.'
							delay={0.2}
						>
							<div className='mb-6 max-w-2xl overflow-hidden rounded-lg border border-zinc-200'>
								<img
									src='/images/agent-os/nersc-server-racks.jpg'
									alt='Server racks at NERSC'
									className='w-full object-cover opacity-90'
									loading='lazy'
								/>
								<p className='bg-zinc-50 px-4 py-2 text-xs text-zinc-600'>
									Server racks at NERSC. Photo by Derrick Coetzee,{' '}
									<a
										href='https://commons.wikimedia.org/w/index.php?curid=17445617'
										className='underline hover:text-zinc-600'
										target='_blank'
										rel='noopener noreferrer'
									>
										CC0
									</a>
								</p>
							</div>
							<div className='grid grid-cols-1 gap-3 sm:grid-cols-2'>
								<PrincipleChip
									label='Model'
									text='Pay for what you use. Scale infinitely. APIs for everything.'
								/>
								<PrincipleChip
									label='Assumption'
									text='Humans write the code. Humans click the buttons. Humans fix the errors.'
								/>
							</div>
						</Era>

						<Era
							year='Now'
							title='The Agent Era'
							lead='AI agents are the new operators. They write code, run commands, fix errors, and deploy software. They work around the clock. They scale to thousands of instances. They don&apos;t need a GUI.'
							body='But agents have different needs than humans. They need persistent memory that survives crashes. They need secure execution environments they can&apos;t escape. They need real-time communication with other agents and systems.'
							future
						>
							<div className='mb-6 max-w-2xl overflow-hidden rounded-lg border border-zinc-200'>
								<img
									src='/images/agent-os/data-flock.jpg'
									alt='Data flock (digits) by Philipp Schmitt'
									className='w-full object-cover opacity-90'
									loading='lazy'
								/>
								<p className='bg-zinc-50 px-4 py-2 text-xs text-zinc-600'>
									"Data flock (digits)" by Philipp Schmitt,{' '}
									<a
										href='https://commons.wikimedia.org/wiki/File:Data_flock_(digits)_by_Philipp_Schmitt.jpg'
										className='underline hover:text-zinc-600'
										target='_blank'
										rel='noopener noreferrer'
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
								className='mb-8 text-lg font-medium text-zinc-900'
							>
								They need an operating system built for them.
							</motion.p>

							<div className='border-t border-zinc-200 pt-6'>
								<div className='mb-4 flex gap-3'>
									<div className='h-6 flex-1 rounded bg-zinc-200' />
									<div className='h-6 flex-[3] rounded bg-zinc-900' />
								</div>
								<div className='flex justify-between text-xs text-zinc-500'>
									<span>Human operators</span>
									<span>AI agents</span>
								</div>
								<p className='mt-3 text-sm text-zinc-500'>
									Soon, more computing tasks will be performed by AI agents than
									by human operators.
								</p>
							</div>
						</Era>
					</div>
				</section>

				{/* CTA */}
				<section className='border-t border-zinc-200 px-6 py-16 md:py-28 text-center'>
					<div className='mx-auto max-w-3xl'>
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className='mb-4 text-2xl font-normal tracking-tight text-zinc-900 md:text-4xl'
						>
							The shift is happening now.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className='mb-8 text-base leading-relaxed text-zinc-500'
						>
							For fifty years, we built operating systems for human operators.
							But the next wave of computing won't be operated by humans.
						</motion.p>
						<motion.a
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							href='/agent-os'
							className='selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-zinc-900 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-zinc-700'
						>
							Explore agentOS
						</motion.a>
					</div>
				</section>
			</main>
		</div>
	);
}

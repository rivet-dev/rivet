'use client';

import { motion } from 'framer-motion';
import { Eyebrow } from '@/components/marketing/editorial/Eyebrow';
import { InkPanel } from '@/components/marketing/editorial/InkPanel';
import { HERO_H1_CLASS, SECTION_H2_CLASS, CAPTION_CLASS } from '@/components/marketing/typography';

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
				className={`font-mono text-sm font-medium ${future ? 'text-pine' : 'text-ink-faint'}`}
			>
				{year}
			</span>
			<div
				className={`hidden h-full w-px md:block ${future ? 'bg-pine/60' : 'bg-pine/25'}`}
			/>
		</div>
		<div className='pb-16'>
			<h2
				className={`mb-4 font-medium tracking-[-0.015em] text-ink ${future ? 'text-3xl md:text-4xl' : 'text-2xl md:text-3xl'}`}
			>
				{title}
			</h2>
			<p className='mb-4 text-base leading-relaxed text-ink-soft md:text-lg'>
				{lead}
			</p>
			{body && (
				<p className='mb-6 text-sm leading-relaxed text-ink-soft md:text-base'>
					{body}
				</p>
			)}
			{children}
		</div>
	</motion.div>
);

// Archival exhibit in the museum mat treatment, with a printed catalog
// caption that keeps the attribution link.
const ArchivalPlate = ({
	src,
	alt,
	figure,
	caption,
	className,
}: {
	src: string;
	alt: string;
	figure: string;
	caption: React.ReactNode;
	className?: string;
}) => (
	<figure className={`max-w-2xl ${className ?? ''}`}>
		<div className='border border-ink/15 bg-mat p-3'>
			<img
				src={src}
				alt={alt}
				className='block h-auto w-full outline outline-1 outline-ink/10'
				loading='lazy'
			/>
		</div>
		<figcaption className={`${CAPTION_CLASS} mt-3 [&_a]:underline [&_a:hover]:text-ink-soft`}>
			<span className='font-medium text-ink-soft'>{figure} — </span>
			{caption}
		</figcaption>
	</figure>
);

const PrincipleChip = ({ label, text }: { label: string; text: string }) => (
	<div className='border border-ink/10 bg-white/55 p-5'>
		<span className='mb-2 block font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-ink-faint'>
			{label}
		</span>
		<p className='text-sm leading-relaxed text-ink-soft'>{text}</p>
	</div>
);

export default function TimelinePage() {
	return (
		<div className='paper-grain min-h-screen overflow-x-hidden font-sans text-ink-soft'>
			<main>
				{/* Hero */}
				<section className='relative flex min-h-[60svh] flex-col items-center justify-center px-6 pt-32'>
					<div className='mx-auto w-full max-w-3xl text-center'>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ duration: 0.5 }}
						>
							<Eyebrow label='agentOS' className='mb-5' />
						</motion.div>
						<motion.h1
							initial={{ opacity: 0, y: 20 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ duration: 0.5 }}
							className={`mb-4 ${HERO_H1_CLASS}`}
						>
							From Unix to Agents
						</motion.h1>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ duration: 0.5, delay: 0.05 }}
							className='text-lg text-ink-soft md:text-xl'
						>
							The operating system is being reinvented. Again.
						</motion.p>
					</div>
				</section>

				{/* Timeline */}
				<section className='border-t border-ink/10 py-16 md:py-32'>
					<div className='mx-auto max-w-7xl px-6'>
						<Era
							year='1969'
							title='The Unix Foundation'
							lead='Before Unix, every computer spoke a different language. Programs written for one machine couldn&apos;t run on another. Computing was fragmented, expensive, and inaccessible.'
							body='Unix changed everything. It introduced a radical idea: a portable operating system with a universal interface. Files, processes, pipes, permissions. Simple primitives that composed into infinite complexity.'
						>
							<ArchivalPlate
								src='https://assets.rivet.dev/website/public/images/agent-os/ken-thompson-dennis-ritchie-1973.jpg'
								alt='Ken Thompson and Dennis Ritchie, creators of Unix, 1973'
								figure='Fig. 01'
								className='mb-6'
								caption={
									<>
										Ken Thompson and Dennis Ritchie, 1973.{' '}
										<a
											href='https://commons.wikimedia.org/w/index.php?curid=31308'
											target='_blank'
											rel='noopener noreferrer'
										>
											Public Domain
										</a>
									</>
								}
							/>
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
							<ArchivalPlate
								src='https://assets.rivet.dev/website/public/images/agent-os/first-web-server.jpg'
								alt='The first web server at CERN'
								figure='Fig. 02'
								className='mt-4'
								caption={
									<>
										The first web server at CERN. Photo by Coolcaesar,{' '}
										<a
											href='https://commons.wikimedia.org/w/index.php?curid=395096'
											target='_blank'
											rel='noopener noreferrer'
										>
											CC BY-SA 3.0
										</a>
									</>
								}
							/>
						</Era>

						<Era
							year='2006'
							title='The Cloud Era'
							lead='AWS, then Azure, then GCP. Computing became a utility. No more buying servers. Just rent capacity by the hour. Infrastructure as code. Scale on demand.'
							body='But the fundamental model stayed the same: humans writing code, humans operating systems, humans in the loop at every step. The cloud made computing elastic, but it was still computing for humans.'
							delay={0.2}
						>
							<ArchivalPlate
								src='https://assets.rivet.dev/website/public/images/agent-os/nersc-server-racks.jpg'
								alt='Server racks at NERSC'
								figure='Fig. 03'
								className='mb-6'
								caption={
									<>
										Server racks at NERSC. Photo by Derrick Coetzee,{' '}
										<a
											href='https://commons.wikimedia.org/w/index.php?curid=17445617'
											target='_blank'
											rel='noopener noreferrer'
										>
											CC0
										</a>
									</>
								}
							/>
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
							<ArchivalPlate
								src='https://assets.rivet.dev/website/public/images/agent-os/data-flock.jpg'
								alt='Data flock (digits) by Philipp Schmitt'
								figure='Fig. 04'
								className='mb-6'
								caption={
									<>
										"Data flock (digits)" by Philipp Schmitt,{' '}
										<a
											href='https://commons.wikimedia.org/wiki/File:Data_flock_(digits)_by_Philipp_Schmitt.jpg'
											target='_blank'
											rel='noopener noreferrer'
										>
											CC BY-SA 4.0
										</a>
									</>
								}
							/>
							<motion.p
								initial={{ opacity: 0 }}
								whileInView={{ opacity: 1 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.2 }}
								className='mb-8 text-lg font-medium text-ink'
							>
								They need an operating system built for them.
							</motion.p>

							<InkPanel
								caption='Fig. 05 — Computing tasks by operator'
								className='max-w-2xl'
							>
								<div className='p-6'>
									<div className='mb-4 flex gap-3'>
										<div className='h-6 flex-1 rounded bg-cream/15' />
										<div className='h-6 flex-[3] rounded bg-sage' />
									</div>
									<div className='flex justify-between font-mono text-[11px] uppercase tracking-[0.16em] text-cream/45'>
										<span>Human operators</span>
										<span>AI agents</span>
									</div>
									<p className='mt-4 text-sm leading-relaxed text-cream/65'>
										Soon, more computing tasks will be performed by AI agents than
										by human operators.
									</p>
								</div>
							</InkPanel>
						</Era>
					</div>
				</section>

				{/* CTA */}
				<section className='border-t border-ink/10 px-6 py-16 text-center md:py-28'>
					<div className='mx-auto max-w-3xl'>
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className={`mb-4 ${SECTION_H2_CLASS}`}
						>
							The shift is happening now.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className='mb-8 text-base leading-relaxed text-ink-soft'
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
							className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-accent-deep px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent'
						>
							Explore agentOS
						</motion.a>
					</div>
				</section>
			</main>
		</div>
	);
}

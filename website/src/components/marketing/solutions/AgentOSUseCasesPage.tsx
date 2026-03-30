'use client';

import { motion } from 'framer-motion';
import {
	ArrowRight,
	Clock,
	Rocket,
	FlaskConical,
	Globe,
	Code,
	User,
	Users,
	Headphones,
	Server,
	Database,
	Pencil,
	Workflow,
	Bot,
	Cpu,
} from 'lucide-react';

interface UseCaseProps {
	icon: React.ComponentType<{ className?: string }>;
	title: string;
	description: string;
	benefits: string[];
	example?: string;
	delay?: number;
}

const UseCase = ({ icon: Icon, title, description, benefits, example, delay = 0 }: UseCaseProps) => (
	<motion.div
		initial={{ opacity: 0, y: 20 }}
		whileInView={{ opacity: 1, y: 0 }}
		viewport={{ once: true }}
		transition={{ duration: 0.5, delay }}
		className='rounded-2xl border border-zinc-200 bg-white p-8 transition-shadow hover:shadow-lg'
	>
		<div className='mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-100'>
			<Icon className='h-6 w-6 text-zinc-700' />
		</div>
		<h3 className='mb-3 text-xl font-medium text-zinc-900'>{title}</h3>
		<p className='mb-4 text-sm leading-relaxed text-zinc-500'>{description}</p>
		<ul className='mb-4 space-y-2'>
			{benefits.map((benefit, i) => (
				<li key={i} className='flex items-start gap-2 text-sm text-zinc-600'>
					<span className='mt-1.5 h-1.5 w-1.5 flex-shrink-0 rounded-full bg-zinc-400' />
					{benefit}
				</li>
			))}
		</ul>
		{example && (
			<div className='rounded-lg bg-zinc-50 px-4 py-3'>
				<span className='text-xs font-medium uppercase tracking-wider text-zinc-400'>Example</span>
				<p className='mt-1 text-sm text-zinc-600'>{example}</p>
			</div>
		)}
	</motion.div>
);

const useCases: UseCaseProps[] = [
	{
		icon: Clock,
		title: 'Background Agents',
		description: 'Long-running agents that operate asynchronously, processing tasks over hours or days without human intervention.',
		benefits: [
			'Persistent state survives crashes and restarts',
			'Queue commands while agents work',
			'Resume from exactly where they left off',
			'Monitor progress in real-time',
		],
		example: 'A code migration agent that refactors a large codebase over several hours, committing changes incrementally.',
	},
	{
		icon: Rocket,
		title: 'Production Agents',
		description: 'Deploy agents at scale with enterprise-grade reliability, security, and observability built in.',
		benefits: [
			'Auto-scaling to thousands of concurrent agents',
			'Built-in health checks and recovery',
			'Comprehensive audit logging',
			'Network isolation and access controls',
		],
		example: 'A fleet of agents processing customer requests 24/7 with guaranteed uptime and automatic failover.',
	},
	{
		icon: FlaskConical,
		title: 'Efficient Evals',
		description: 'Run agent evaluations and benchmarks at scale without spinning up expensive sandboxes for each test.',
		benefits: [
			'75x lower memory per instance than sandboxes',
			'Millisecond cold starts for rapid iteration',
			'Deterministic replay for debugging',
			'Cost-effective at thousands of runs',
		],
		example: 'Evaluating 10,000 agent responses in parallel to measure performance across different prompts.',
	},
	{
		icon: Globe,
		title: 'Browser-based LLM Interfaces',
		description: 'Build interactive web applications where agents execute code and manage state in real-time.',
		benefits: [
			'WebSocket streaming for live updates',
			'Preview URLs for agent-generated apps',
			'Secure execution in isolated environments',
			'Session persistence across page reloads',
		],
		example: 'An AI coding assistant that runs in the browser, executing code and showing live previews.',
	},
	{
		icon: Code,
		title: 'Programming Agents',
		description: 'Purpose-built for agents that write, test, and deploy code autonomously.',
		benefits: [
			'Native file system access with git support',
			'Shell execution with full toolchain access',
			'Package installation and dependency management',
			'Test runner integration',
		],
		example: 'An agent that takes a GitHub issue, writes the fix, runs tests, and opens a pull request.',
	},
	{
		icon: User,
		title: 'Personal Agents',
		description: 'Lightweight agents that assist individual users with daily tasks and workflows.',
		benefits: [
			'Low resource overhead for personal use',
			'Local-first with optional cloud sync',
			'Custom tool integration',
			'Privacy-focused execution',
		],
		example: 'A personal agent that organizes your calendar, drafts emails, and manages your todo list.',
	},
	{
		icon: Users,
		title: 'Multi-Agent Systems',
		description: 'Coordinate multiple agents working together on complex tasks with shared state and communication.',
		benefits: [
			'Shared file systems between agents',
			'Real-time inter-agent messaging',
			'Workflow orchestration primitives',
			'Centralized observability',
		],
		example: 'A team of agents where one researches, one writes, and one reviews, all collaborating on a document.',
	},
	{
		icon: Headphones,
		title: 'Customer Support Agents',
		description: 'Deploy agents that handle customer inquiries with access to your systems and data.',
		benefits: [
			'Database and API integration',
			'Secure credential management',
			'Audit trails for compliance',
			'Human handoff capabilities',
		],
		example: 'An agent that resolves billing issues by checking accounts, applying credits, and sending confirmations.',
	},
	{
		icon: Server,
		title: 'DevOps Agents',
		description: 'Agents that manage infrastructure, deployments, and incident response autonomously.',
		benefits: [
			'SSH and cloud provider access',
			'Infrastructure as code execution',
			'Automated incident remediation',
			'Deployment pipeline integration',
		],
		example: 'An on-call agent that detects issues, diagnoses root causes, and applies fixes automatically.',
	},
	{
		icon: Database,
		title: 'Data Processing Agents',
		description: 'Run ETL pipelines, data transformations, and analysis tasks with agent intelligence.',
		benefits: [
			'Stream processing capabilities',
			'Database connections and queries',
			'File format conversion',
			'Incremental processing',
		],
		example: 'An agent that ingests raw data, cleans it, runs analysis, and generates reports on a schedule.',
	},
	{
		icon: Pencil,
		title: 'Content Creation Agents',
		description: 'Agents that create, edit, and publish content across multiple platforms.',
		benefits: [
			'Multi-modal content support',
			'Publishing API integrations',
			'Asset management',
			'Version control for drafts',
		],
		example: 'An agent that writes blog posts, generates images, and publishes to your CMS automatically.',
	},
	{
		icon: Workflow,
		title: 'Workflow Automation',
		description: 'Chain agent tasks into complex workflows with conditional logic and human-in-the-loop steps.',
		benefits: [
			'Durable workflow execution',
			'Retry and error handling',
			'Scheduled and triggered runs',
			'Approval gates and notifications',
		],
		example: 'A hiring workflow where agents screen resumes, schedule interviews, and prepare onboarding docs.',
	},
	{
		icon: Cpu,
		title: 'Edge & Embedded Agents',
		description: 'Run agents on resource-constrained devices like Raspberry Pi, edge servers, or IoT gateways.',
		benefits: [
			'~3.4 MB memory footprint per agent',
			'No containers or VMs required',
			'Works offline with local LLMs',
			'ARM and x86 support',
		],
		example: 'A home automation agent running on a Raspberry Pi that controls smart devices and responds to voice commands.',
	},
];

export default function AgentOSUseCasesPage() {
	return (
		<div className='min-h-screen bg-white font-sans text-zinc-600 selection:bg-zinc-200 selection:text-zinc-900'>
			<main>
				{/* Hero */}
				<section className='relative flex min-h-[50svh] flex-col items-center justify-center px-6 pt-32'>
					<div className='mx-auto w-full max-w-4xl text-center'>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ duration: 0.5 }}
							className='mb-6 flex items-center justify-center gap-3'
						>
							<a href='/agent-os' className='text-sm text-zinc-400 hover:text-zinc-600 transition-colors'>
								agentOS
							</a>
							<span className='text-zinc-300'>/</span>
							<span className='text-sm text-zinc-900'>Use Cases</span>
						</motion.div>
						<motion.h1
							initial={{ opacity: 0, y: 20 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ duration: 0.5, delay: 0.05 }}
							className='mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-zinc-900 md:text-6xl'
						>
							Who is agentOS for?
						</motion.h1>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className='mx-auto max-w-2xl text-lg text-zinc-500 md:text-xl'
						>
							From personal assistants to enterprise fleets, agentOS powers every kind of AI agent.
						</motion.p>
					</div>
				</section>

				{/* Use Cases Grid */}
				<section className='border-t border-zinc-200 px-6 py-16 md:py-24'>
					<div className='mx-auto max-w-7xl'>
						<div className='grid gap-6 md:grid-cols-2 lg:grid-cols-3'>
							{useCases.map((useCase, i) => (
								<UseCase key={useCase.title} {...useCase} delay={i * 0.05} />
							))}
						</div>
					</div>
				</section>

				{/* CTA */}
				<section className='border-t border-zinc-200 px-6 py-16 md:py-24'>
					<div className='mx-auto max-w-3xl text-center'>
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className='mb-4 text-2xl font-normal tracking-tight text-zinc-900 md:text-4xl'
						>
							Ready to build?
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className='mb-8 text-base leading-relaxed text-zinc-500'
						>
							Get started with agentOS in minutes. One npm install, zero infrastructure.
						</motion.p>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className='flex flex-col items-center justify-center gap-4 sm:flex-row'
						>
							<a
								href='/docs'
								className='selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-zinc-900 px-6 py-3 text-sm font-medium text-white transition-colors hover:bg-zinc-700'
							>
								Read the Docs
								<ArrowRight className='h-4 w-4' />
							</a>
							<a
								href='/agent-os'
								className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-zinc-300 px-6 py-3 text-sm font-medium text-zinc-700 transition-colors hover:border-zinc-400 hover:text-zinc-900'
							>
								<Bot className='h-4 w-4' />
								Explore agentOS
							</a>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}

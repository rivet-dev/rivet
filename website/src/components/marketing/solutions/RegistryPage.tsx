'use client';

import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
	Search,
	FolderOpen,
	Database,
	Globe,
	Terminal,
	Shield,
	Package,
	Code,
	Zap,
	GitBranch,
	Cloud,
	Lock,
	Cpu,
	HardDrive,
	Network,
	FileCode,
	Box,
	Check,
	Copy,
} from 'lucide-react';

// --- Registry Item Data ---
interface RegistryItem {
	name: string;
	description: string;
	category: string;
	icon: React.ComponentType<{ className?: string }>;
	downloads: string;
	version: string;
	featured?: boolean;
}

const registryItems: RegistryItem[] = [
	// File Systems
	{ name: 's3-filesystem', description: 'Mount S3 buckets as a virtual file system for agents', category: 'File Systems', icon: FolderOpen, downloads: '12.4k', version: '1.2.0', featured: true },
	{ name: 'google-drive-fs', description: 'Mount Google Drive as a file system', category: 'File Systems', icon: FolderOpen, downloads: '6.3k', version: '1.0.2' },
	{ name: 'azure-blob-fs', description: 'Mount Azure Blob Storage as a file system', category: 'File Systems', icon: Cloud, downloads: '3.8k', version: '1.0.1' },
	{ name: 'dropbox-fs', description: 'Mount Dropbox folders as a file system', category: 'File Systems', icon: FolderOpen, downloads: '2.1k', version: '0.9.0' },
	{ name: 'local-fs', description: 'Secure access to host file system with sandboxing', category: 'File Systems', icon: HardDrive, downloads: '18.2k', version: '2.0.0' },

	// Databases
	{ name: 'sqlite-driver', description: 'SQLite database driver with full SQL support', category: 'Databases', icon: Database, downloads: '8.7k', version: '2.0.1', featured: true },
	{ name: 'postgres-driver', description: 'PostgreSQL database driver with connection pooling', category: 'Databases', icon: Database, downloads: '9.8k', version: '1.3.0' },
	{ name: 'redis-cache', description: 'Redis caching layer for agent state', category: 'Databases', icon: Zap, downloads: '5.9k', version: '1.0.0' },
	{ name: 'mongodb-driver', description: 'MongoDB driver with full document support', category: 'Databases', icon: Database, downloads: '4.2k', version: '1.1.0' },
	{ name: 'mysql-driver', description: 'MySQL and MariaDB database driver', category: 'Databases', icon: Database, downloads: '3.5k', version: '1.0.0' },

	// Integrations
	{ name: 'github-integration', description: 'Clone, commit, and push to GitHub repositories', category: 'Integrations', icon: GitBranch, downloads: '15.2k', version: '1.5.3', featured: true },
	{ name: 'vercel-deploy', description: 'Deploy to Vercel directly from agent workflows', category: 'Integrations', icon: Globe, downloads: '7.2k', version: '2.1.0' },
	{ name: 'slack-notifications', description: 'Send notifications and updates to Slack channels', category: 'Integrations', icon: Globe, downloads: '4.8k', version: '1.2.0' },
	{ name: 'linear-integration', description: 'Create and manage Linear issues from agents', category: 'Integrations', icon: Box, downloads: '2.9k', version: '1.0.0' },
	{ name: 'notion-connector', description: 'Read and write to Notion databases and pages', category: 'Integrations', icon: FileCode, downloads: '3.4k', version: '1.1.0' },

	// Tools
	{ name: 'shell-executor', description: 'Secure shell command execution with sandboxing', category: 'Tools', icon: Terminal, downloads: '21.1k', version: '3.0.0', featured: true },
	{ name: 'docker-runner', description: 'Run Docker containers within agent sessions', category: 'Tools', icon: Box, downloads: '11.3k', version: '1.4.2' },
	{ name: 'code-formatter', description: 'Format code with Prettier, Black, and more', category: 'Tools', icon: Code, downloads: '6.7k', version: '1.2.0' },
	{ name: 'test-runner', description: 'Run tests with Jest, Pytest, and other frameworks', category: 'Tools', icon: Cpu, downloads: '5.4k', version: '1.0.0' },
	{ name: 'package-manager', description: 'Install npm, pip, and cargo packages securely', category: 'Tools', icon: Package, downloads: '14.2k', version: '2.0.0' },

	// Security
	{ name: 'network-proxy', description: 'Configurable network proxy with allowlist support', category: 'Security', icon: Network, downloads: '4.5k', version: '1.1.0' },
	{ name: 'secrets-manager', description: 'Secure secrets injection from various providers', category: 'Security', icon: Lock, downloads: '8.1k', version: '2.0.0' },
	{ name: 'rate-limiter', description: 'Rate limiting for API calls and resource usage', category: 'Security', icon: Shield, downloads: '3.2k', version: '1.0.0' },
	{ name: 'audit-logger', description: 'Comprehensive audit logging for compliance', category: 'Security', icon: FileCode, downloads: '2.8k', version: '1.0.0' },

	// Sandboxes
	{ name: 'e2b-sandbox', description: 'Run agents in E2B cloud sandboxes', category: 'Sandboxes', icon: Cloud, downloads: '9.2k', version: '1.2.0', featured: true },
	{ name: 'modal-sandbox', description: 'Execute code in Modal serverless containers', category: 'Sandboxes', icon: Box, downloads: '6.8k', version: '1.0.0' },
	{ name: 'fly-machines', description: 'Spin up Fly.io machines for isolated execution', category: 'Sandboxes', icon: Globe, downloads: '4.1k', version: '1.1.0' },
	{ name: 'docker-sandbox', description: 'Local Docker container sandboxing', category: 'Sandboxes', icon: Box, downloads: '12.5k', version: '2.0.0' },
	{ name: 'firecracker-vm', description: 'Firecracker microVM isolation for high security', category: 'Sandboxes', icon: Shield, downloads: '3.4k', version: '0.9.0' },
];

const categories = ['All', 'Sandboxes', 'File Systems', 'Databases', 'Integrations', 'Tools', 'Security'];

// --- Copy Button ---
const CopyButton = ({ text }: { text: string }) => {
	const [copied, setCopied] = useState(false);

	const handleCopy = async () => {
		await navigator.clipboard.writeText(text);
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	};

	return (
		<button
			onClick={handleCopy}
			className='flex h-8 w-8 items-center justify-center rounded-lg bg-zinc-100 text-zinc-500 transition-colors hover:bg-zinc-200 hover:text-zinc-700'
		>
			{copied ? <Check className='h-4 w-4 text-green-600' /> : <Copy className='h-4 w-4' />}
		</button>
	);
};

// --- Hero ---
const Hero = () => (
	<section className='relative px-6 pt-32 pb-16 md:pt-40 md:pb-24'>
		<div className='mx-auto max-w-5xl'>
			<motion.div
				initial={{ opacity: 0, y: 20 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{ duration: 0.5 }}
				className='text-center'
			>
				<h1 className='mb-6 text-4xl font-normal tracking-tight text-zinc-900 md:text-6xl'>
					AgentOS Registry
				</h1>
				<p className='mx-auto max-w-2xl text-lg text-zinc-500'>
					Pre-built tools, integrations, and capabilities for your agents.<br />
					From file systems to databases to API connectors.
				</p>
			</motion.div>
		</div>
	</section>
);

// --- Search and Filter ---
const SearchAndFilter = ({
	search,
	setSearch,
	activeCategory,
	setActiveCategory,
}: {
	search: string;
	setSearch: (s: string) => void;
	activeCategory: string;
	setActiveCategory: (c: string) => void;
}) => (
	<div className='mx-auto max-w-5xl px-6'>
		<motion.div
			initial={{ opacity: 0, y: 20 }}
			animate={{ opacity: 1, y: 0 }}
			transition={{ duration: 0.5, delay: 0.1 }}
			className='mb-8'
		>
			{/* Search */}
			<div className='relative mb-6'>
				<Search className='absolute left-4 top-1/2 h-5 w-5 -translate-y-1/2 text-zinc-400' />
				<input
					type='text'
					placeholder='Search packages...'
					value={search}
					onChange={(e) => setSearch(e.target.value)}
					className='w-full rounded-xl border border-zinc-200 bg-white py-4 pl-12 pr-4 text-zinc-900 placeholder-zinc-400 outline-none transition-colors focus:border-zinc-400'
				/>
			</div>

			{/* Categories */}
			<div className='flex flex-wrap gap-2'>
				{categories.map((cat) => (
					<button
						key={cat}
						onClick={() => setActiveCategory(cat)}
						className={`relative rounded-lg px-4 py-2 text-sm transition-colors ${
							activeCategory === cat
								? 'text-white'
								: 'bg-zinc-100 text-zinc-600 hover:bg-zinc-200'
						}`}
					>
						{activeCategory === cat && (
							<motion.div
								layoutId='activeCategory'
								className='absolute inset-0 rounded-lg bg-zinc-900'
								transition={{ type: 'spring', bounce: 0.2, duration: 0.4 }}
							/>
						)}
						<span className='relative z-10'>{cat}</span>
					</button>
				))}
			</div>
		</motion.div>
	</div>
);

// --- Package Card ---
const PackageCard = ({ item }: { item: RegistryItem }) => {
	const Icon = item.icon;
	const installCommand = `npm install @agentos/${item.name}`;

	return (
		<div
			className='group relative rounded-2xl border border-zinc-200 bg-white p-6 transition-all hover:border-zinc-300 hover:shadow-lg'
		>
			<div className='mb-4 flex items-start justify-between'>
				<div className='flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-100'>
					<Icon className='h-6 w-6 text-zinc-600' />
				</div>
			</div>
			<h3 className='mb-1 font-mono text-base font-medium text-zinc-900'>
				{item.name}
			</h3>
			<p className='mb-4 text-sm leading-relaxed text-zinc-500'>
				{item.description}
			</p>

			{/* Install command */}
			<div className='flex items-center gap-2 rounded-lg bg-zinc-50 px-3 py-2'>
				<code className='flex-1 truncate font-mono text-xs text-zinc-600'>
					{installCommand}
				</code>
				<CopyButton text={installCommand} />
			</div>
		</div>
	);
};

// --- Package Grid ---
const PackageGrid = ({ items }: { items: RegistryItem[] }) => (
	<div className='mx-auto max-w-5xl px-6 pb-24'>
		<AnimatePresence mode='wait'>
			{items.length === 0 ? (
				<motion.div
					key='empty'
					initial={{ opacity: 0 }}
					animate={{ opacity: 1 }}
					exit={{ opacity: 0 }}
					className='py-16 text-center text-zinc-500'
				>
					No packages found matching your criteria.
				</motion.div>
			) : (
				<motion.div
					key='grid'
					initial={{ opacity: 0 }}
					animate={{ opacity: 1 }}
					exit={{ opacity: 0 }}
					className='grid gap-6 md:grid-cols-2 lg:grid-cols-3'
				>
					{items.map((item) => (
						<PackageCard key={item.name} item={item} />
					))}
				</motion.div>
			)}
		</AnimatePresence>
	</div>
);

// --- Main Page ---
export default function RegistryPage() {
	const [search, setSearch] = useState('');
	const [activeCategory, setActiveCategory] = useState('All');

	const filteredItems = registryItems.filter((item) => {
		const matchesSearch =
			item.name.toLowerCase().includes(search.toLowerCase()) ||
			item.description.toLowerCase().includes(search.toLowerCase());
		const matchesCategory =
			activeCategory === 'All' || item.category === activeCategory;
		return matchesSearch && matchesCategory;
	});

	return (
		<div className='min-h-screen bg-white font-sans text-zinc-600 selection:bg-zinc-200 selection:text-zinc-900'>
			<main>
				<Hero />
				<SearchAndFilter
					search={search}
					setSearch={setSearch}
					activeCategory={activeCategory}
					setActiveCategory={setActiveCategory}
				/>
				<PackageGrid items={filteredItems} />
			</main>
		</div>
	);
}

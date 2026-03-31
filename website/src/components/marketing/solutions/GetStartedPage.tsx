'use client';

import { useState, useEffect, useRef, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Check, Copy, ArrowRight } from 'lucide-react';

// --- Conway's Game of Life Background ---
const GameOfLife = () => {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const gridRef = useRef<boolean[][]>([]);
	const animationRef = useRef<number>();

	const CELL_SIZE = 16;
	const UPDATE_INTERVAL = 150;

	const initGrid = useCallback((cols: number, rows: number) => {
		const grid: boolean[][] = [];
		for (let i = 0; i < rows; i++) {
			grid[i] = [];
			for (let j = 0; j < cols; j++) {
				// Sparse random initialization
				grid[i][j] = Math.random() < 0.15;
			}
		}
		return grid;
	}, []);

	const countNeighbors = useCallback((grid: boolean[][], x: number, y: number, rows: number, cols: number) => {
		let count = 0;
		for (let i = -1; i <= 1; i++) {
			for (let j = -1; j <= 1; j++) {
				if (i === 0 && j === 0) continue;
				const ni = (y + i + rows) % rows;
				const nj = (x + j + cols) % cols;
				if (grid[ni][nj]) count++;
			}
		}
		return count;
	}, []);

	const nextGeneration = useCallback((grid: boolean[][], rows: number, cols: number) => {
		const newGrid: boolean[][] = [];
		for (let i = 0; i < rows; i++) {
			newGrid[i] = [];
			for (let j = 0; j < cols; j++) {
				const neighbors = countNeighbors(grid, j, i, rows, cols);
				if (grid[i][j]) {
					newGrid[i][j] = neighbors === 2 || neighbors === 3;
				} else {
					newGrid[i][j] = neighbors === 3;
				}
			}
		}
		return newGrid;
	}, [countNeighbors]);

	useEffect(() => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		const resize = () => {
			canvas.width = canvas.offsetWidth;
			canvas.height = canvas.offsetHeight;
			const cols = Math.ceil(canvas.width / CELL_SIZE);
			const rows = Math.ceil(canvas.height / CELL_SIZE);
			gridRef.current = initGrid(cols, rows);
		};

		resize();
		window.addEventListener('resize', resize);

		let lastUpdate = 0;

		const draw = (timestamp: number) => {
			if (!ctx || !canvas) return;

			if (timestamp - lastUpdate > UPDATE_INTERVAL) {
				const cols = Math.ceil(canvas.width / CELL_SIZE);
				const rows = Math.ceil(canvas.height / CELL_SIZE);

				// Occasionally add new cells to keep it alive
				if (Math.random() < 0.02) {
					const rx = Math.floor(Math.random() * cols);
					const ry = Math.floor(Math.random() * rows);
					for (let i = -1; i <= 1; i++) {
						for (let j = -1; j <= 1; j++) {
							const ni = (ry + i + rows) % rows;
							const nj = (rx + j + cols) % cols;
							if (gridRef.current[ni] && Math.random() < 0.5) {
								gridRef.current[ni][nj] = true;
							}
						}
					}
				}

				gridRef.current = nextGeneration(gridRef.current, rows, cols);
				lastUpdate = timestamp;
			}

			ctx.clearRect(0, 0, canvas.width, canvas.height);

			const cols = Math.ceil(canvas.width / CELL_SIZE);
			const rows = Math.ceil(canvas.height / CELL_SIZE);

			for (let i = 0; i < rows; i++) {
				for (let j = 0; j < cols; j++) {
					if (gridRef.current[i]?.[j]) {
						ctx.fillStyle = 'rgba(228, 228, 231, 0.6)'; // zinc-200 with transparency
						ctx.fillRect(
							j * CELL_SIZE + 1,
							i * CELL_SIZE + 1,
							CELL_SIZE - 2,
							CELL_SIZE - 2
						);
					}
				}
			}

			animationRef.current = requestAnimationFrame(draw);
		};

		animationRef.current = requestAnimationFrame(draw);

		return () => {
			window.removeEventListener('resize', resize);
			if (animationRef.current) {
				cancelAnimationFrame(animationRef.current);
			}
		};
	}, [initGrid, nextGeneration]);

	return (
		<canvas
			ref={canvasRef}
			className='absolute inset-0 w-full h-full'
			style={{ opacity: 0.8 }}
		/>
	);
};

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
		<div className='relative flex min-h-screen flex-col items-center justify-center bg-white selection:bg-zinc-200 selection:text-zinc-900'>
			{/* Game of Life Background */}
			<div className='absolute inset-0 z-0'>
				<GameOfLife />
			</div>
			{/* Hero */}
			<section className='relative z-10 px-6'>
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
								alt='agentOS'
								className='h-16 w-auto md:h-20'
							/>

						</div>
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
							className='inline-flex items-center justify-center gap-3 rounded-md bg-zinc-900 px-6 py-3 text-sm font-medium text-white transition-colors hover:bg-zinc-700'
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

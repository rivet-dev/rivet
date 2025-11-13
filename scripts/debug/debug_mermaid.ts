#!/usr/bin/env tsx

/**
 * Binary search to find problematic Mermaid gantt entries
 * Usage: tsx scripts/debug/debug_mermaid.ts
 */

import { readFileSync, writeFileSync } from 'fs';

function main() {
	const inputFile = '/tmp/rivet-engine-gantt.md';
	const content = readFileSync(inputFile, 'utf-8');
	const lines = content.split('\n');

	// Find the start and end of the mermaid block
	const startIdx = lines.findIndex(line => line.trim() === '```mermaid');
	const endIdx = lines.findIndex((line, idx) => idx > startIdx && line.trim() === '```');

	if (startIdx === -1 || endIdx === -1) {
		console.error('Could not find mermaid block');
		process.exit(1);
	}

	const mermaidLines = lines.slice(startIdx + 1, endIdx);

	// Find where sections start
	const sectionStarts: number[] = [];
	mermaidLines.forEach((line, idx) => {
		if (line.trim().startsWith('section ')) {
			sectionStarts.push(idx);
		}
	});

	console.log(`Found ${sectionStarts.length} sections`);

	// Try with just the header
	console.log('\nTesting with just header...');
	let testLines = mermaidLines.slice(0, Math.max(...sectionStarts.filter(s => s < 10)));
	writeTestFile(testLines, '/tmp/test-gantt.md');
	console.log('Created /tmp/test-gantt.md with just headers');

	// Try with first section only
	console.log('\nTesting with first section only...');
	const firstSectionEnd = sectionStarts[1] || mermaidLines.length;
	testLines = mermaidLines.slice(0, firstSectionEnd);
	writeTestFile(testLines, '/tmp/test-gantt-section1.md');
	console.log('Created /tmp/test-gantt-section1.md with first section');

	// Create files with incremental sections
	for (let i = 0; i < sectionStarts.length; i++) {
		const endLine = i + 1 < sectionStarts.length ? sectionStarts[i + 1] : mermaidLines.length;
		testLines = mermaidLines.slice(0, endLine);
		writeTestFile(testLines, `/tmp/test-gantt-${i + 1}sections.md`);
		console.log(`Created /tmp/test-gantt-${i + 1}sections.md`);
	}

	// Also create a minimal test
	console.log('\nCreating minimal test...');
	const minimal = [
		'gantt',
		'    title Test',
		'    dateFormat YYYY-MM-DDTHH:mm:ss',
		'    axisFormat %H:%M:%S',
		'    section Test Section',
		'    task1 :a1, 2025-11-12T22:30:21, 2025-11-12T22:30:22',
		'    task2 :crit, a2, 2025-11-12T22:30:22, 2025-11-12T22:30:23',
		'    task3 :active, a3, 2025-11-12T22:30:23, 2025-11-12T22:30:24',
		'    task4 :milestone, a4, 2025-11-12T22:30:24, 2025-11-12T22:30:24',
	];
	writeTestFile(minimal, '/tmp/test-gantt-minimal.md');
	console.log('Created /tmp/test-gantt-minimal.md');

	// Print first few task lines for inspection
	console.log('\nFirst 10 task lines:');
	let taskCount = 0;
	for (const line of mermaidLines) {
		if (line.trim() && !line.trim().startsWith('gantt') && !line.trim().startsWith('title') &&
		    !line.trim().startsWith('dateFormat') && !line.trim().startsWith('axisFormat') &&
		    !line.trim().startsWith('section')) {
			console.log(`  ${line}`);
			taskCount++;
			if (taskCount >= 10) break;
		}
	}
}

function writeTestFile(mermaidLines: string[], outputPath: string) {
	const output = [
		'# Test Mermaid Gantt',
		'',
		'```mermaid',
		...mermaidLines,
		'```'
	].join('\n');
	writeFileSync(outputPath, output);
}

if (require.main === module) {
	main();
}

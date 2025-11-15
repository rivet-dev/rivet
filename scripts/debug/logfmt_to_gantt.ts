#!/usr/bin/env tsx

/**
 * Parses a logfmt log file and generates a Mermaid Gantt chart
 * Usage: tsx scripts/debug/logfmt_to_gantt.ts [input_file] [output_file]
 */

import { readFileSync, writeFileSync } from 'fs';

interface LogEntry {
	ts: Date;
	tsFracSec: number; // Fractional seconds for sub-millisecond precision
	level: string;
	message: string;
	workflow_id?: string;
	workflow_name?: string;
	activity_name?: string;
	signal_name?: string;
	service?: string;
	operation_name?: string;
	ray_id?: string;
	iteration?: number;
	location?: string;
	[key: string]: any;
}

interface Task {
	id: string;
	name: string;
	start: Date;
	startFracSec: number;
	end?: Date;
	endFracSec?: number;
	section: string;
	type: 'workflow' | 'activity' | 'operation' | 'service' | 'signal';
	metadata: Record<string, any>;
}

// ANSI color code regex
const ANSI_REGEX = /\x1b\[[0-9;]*m/g;

function stripAnsi(str: string): string {
	return str.replace(ANSI_REGEX, '');
}

function parseLogfmt(line: string): LogEntry | null {
	const cleaned = stripAnsi(line);
	const entry: Partial<LogEntry> = {};

	// Match key=value pairs, handling quoted values
	const regex = /(\w+)=(?:"([^"]*)"|(\S+))/g;
	let match;

	while ((match = regex.exec(cleaned)) !== null) {
		const key = match[1];
		const value = match[2] !== undefined ? match[2] : match[3];

		if (key === 'ts') {
			entry.ts = new Date(value);
			const fracMatch = value.match(/\.(\d+)/);
			if (fracMatch) {
				const fracStr = fracMatch[1];
				if (fracStr.length > 3) {
					entry.tsFracSec = parseFloat('0.000' + fracStr.substring(3));
				} else {
					entry.tsFracSec = 0;
				}
			} else {
				entry.tsFracSec = 0;
			}
		} else if (key === 'level') {
			entry.level = value;
		} else if (key === 'message') {
			entry.message = value;
		} else {
			entry[key] = value;
		}
	}

	if (!entry.ts || !entry.message) {
		return null;
	}

	return entry as LogEntry;
}

function sanitizeForMermaid(str: string): string {
	// Remove special characters that might break Mermaid syntax
	return str
		.replace(/[:#,]/g, '_')
		.replace(/\s+/g, '_')
		.replace(/[{}[\]()]/g, '')
		.replace(/_+/g, '_') // Replace multiple underscores with single
		.replace(/^_|_$/g, '') // Remove leading/trailing underscores
		.substring(0, 50); // Limit length
}

function formatDuration(ms: number): string {
	if (ms < 1) return `${(ms * 1000).toFixed(0)}μs`;
	if (ms < 1000) return `${ms.toFixed(2)}ms`;
	return `${(ms / 1000).toFixed(2)}s`;
}

function main() {
	const args = process.argv.slice(2);

	// Check for --split-by-workflow flag
	const splitByWorkflow = args.includes('--split-by-workflow');

	// Check for time filter arguments
	const startTimeIdx = args.findIndex(arg => arg === '--start-time');
	const endTimeIdx = args.findIndex(arg => arg === '--end-time');

	let startTimeFilter: Date | null = null;
	let endTimeFilter: Date | null = null;

	if (startTimeIdx !== -1 && args[startTimeIdx + 1]) {
		const timeStr = args[startTimeIdx + 1];
		// Parse time in format HH:mm:ss or full ISO timestamp
		if (timeStr.includes('T')) {
			startTimeFilter = new Date(timeStr);
		} else {
			// Assume it's a time today, use a placeholder date that we'll replace
			startTimeFilter = new Date(`2000-01-01T${timeStr}Z`);
		}
	}

	if (endTimeIdx !== -1 && args[endTimeIdx + 1]) {
		const timeStr = args[endTimeIdx + 1];
		if (timeStr.includes('T')) {
			endTimeFilter = new Date(timeStr);
		} else {
			endTimeFilter = new Date(`2000-01-01T${timeStr}Z`);
		}
	}

	const filteredArgs = args.filter(arg =>
		arg !== '--split-by-workflow' &&
		arg !== '--start-time' &&
		arg !== '--end-time' &&
		!arg.match(/^\d{2}:\d{2}:\d{2}/) &&
		!arg.match(/^\d{4}-\d{2}-\d{2}T/)
	);

	const inputFile = filteredArgs[0] || '/tmp/rivet-engine.log';
	const outputFile = filteredArgs[1] || '/tmp/rivet-engine-gantt.md';

	console.log(`Reading log file: ${inputFile}`);
	if (splitByWorkflow) {
		console.log('Mode: Split by workflow (entries without workflow_id will be excluded)');
	} else {
		console.log('Mode: Single section');
	}
	if (startTimeFilter || endTimeFilter) {
		console.log(`Time filter: ${startTimeFilter ? startTimeFilter.toISOString().substring(11, 19) : 'start'} to ${endTimeFilter ? endTimeFilter.toISOString().substring(11, 19) : 'end'}`);
	}
	const content = readFileSync(inputFile, 'utf-8');
	const lines = content.split('\n');

	const entries: LogEntry[] = [];
	for (const line of lines) {
		if (!line.trim()) continue;
		const entry = parseLogfmt(line);
		if (entry) {
			entries.push(entry);
		}
	}

	console.log(`Parsed ${entries.length} log entries`);

	if (entries.length === 0) {
		console.error('No valid log entries found');
		process.exit(1);
	}

	// Apply time filters
	if (startTimeFilter || endTimeFilter) {
		const originalCount = entries.length;

		// If using HH:mm:ss format, we need to match against the time portion
		const useTimeOfDay = startTimeFilter && startTimeFilter.getFullYear() === 2000;

		if (useTimeOfDay) {
			// Extract time of day from the placeholder dates
			const startTimeOfDay = startTimeFilter ? startTimeFilter.getUTCHours() * 3600 + startTimeFilter.getUTCMinutes() * 60 + startTimeFilter.getUTCSeconds() : 0;
			const endTimeOfDay = endTimeFilter ? endTimeFilter.getUTCHours() * 3600 + endTimeFilter.getUTCMinutes() * 60 + endTimeFilter.getUTCSeconds() : 86400;

			entries.splice(0, entries.length, ...entries.filter(entry => {
				const entryTimeOfDay = entry.ts.getUTCHours() * 3600 + entry.ts.getUTCMinutes() * 60 + entry.ts.getUTCSeconds();
				return entryTimeOfDay >= startTimeOfDay && entryTimeOfDay <= endTimeOfDay;
			}));
		} else {
			// Use full timestamp comparison
			entries.splice(0, entries.length, ...entries.filter(entry => {
				const entryTime = entry.ts.getTime();
				const afterStart = !startTimeFilter || entryTime >= startTimeFilter.getTime();
				const beforeEnd = !endTimeFilter || entryTime <= endTimeFilter.getTime();
				return afterStart && beforeEnd;
			}));
		}

		console.log(`Filtered to ${entries.length} entries (removed ${originalCount - entries.length})`);

		if (entries.length === 0) {
			console.error('No entries match the time filter');
			process.exit(1);
		}
	}

	// Track tasks by workflow_id
	const workflowExecutionTasks: Map<string, Task> = new Map();
	const activityTasks: Map<string, Task> = new Map();
	const operationTasks: Map<string, Task> = new Map();
	const signalTasks: Map<string, Task> = new Map();

	// Track workflow metadata for section naming
	const workflowMetadata: Map<string, { name: string, id: string }> = new Map();

	// Track active workflow executions (to pair running/sleeping)
	const activeWorkflowExecutions: Map<string, { start: Date, startFracSec: number, count: number }> = new Map();

	// Track signal send times (for calculating recv duration)
	const signalSendTimes: Map<string, { ts: Date, tsFracSec: number, signal_name: string }> = new Map();

	const startTime = entries[0].ts;

	for (const entry of entries) {
		// Track workflow metadata and execution segments
		if (entry.workflow_id && entry.workflow_name) {
			if (!workflowMetadata.has(entry.workflow_id)) {
				workflowMetadata.set(entry.workflow_id, {
					name: entry.workflow_name,
					id: entry.workflow_id
				});
			}

			// Track workflow execution segments (running -> sleeping/completed)
			if (entry.message === 'running workflow') {
				const wfKey = entry.workflow_id;
				// Track how many times this workflow has run
				const execInfo = activeWorkflowExecutions.get(wfKey);
				const execCount = execInfo ? execInfo.count + 1 : 0;

				activeWorkflowExecutions.set(wfKey, {
					start: entry.ts,
					startFracSec: entry.tsFracSec,
					count: execCount
				});
			} else if (entry.message === 'workflow sleeping' || entry.message === 'workflow completed' || entry.message === 'workflow error') {
				const wfKey = entry.workflow_id;
				const execInfo = activeWorkflowExecutions.get(wfKey);

				if (execInfo) {
					const taskKey = `${wfKey}_exec_${execInfo.count}`;
					workflowExecutionTasks.set(taskKey, {
						id: sanitizeForMermaid(`wf_exec_${taskKey}`),
						name: `${entry.workflow_name} exec`,
						start: execInfo.start,
						startFracSec: execInfo.startFracSec,
						end: entry.ts,
						endFracSec: entry.tsFracSec,
						section: `${entry.workflow_name} (${entry.workflow_id})`,
						type: 'workflow',
						metadata: { workflow_id: entry.workflow_id, ray_id: entry.ray_id, exec_count: execInfo.count }
					});
				}
			}
		}

		// Track activity execution
		if (entry.activity_name && entry.workflow_id) {
			const locationStr = entry.location ? `_${sanitizeForMermaid(entry.location)}` : '';
			const iterStr = entry.iteration !== undefined ? `_i${entry.iteration}` : '';
			const key = `${entry.workflow_id}_${entry.activity_name}${locationStr}${iterStr}`;

			if (entry.message === 'running activity') {
				const wfMeta = workflowMetadata.get(entry.workflow_id);
				const sectionName = wfMeta ? `${wfMeta.name} (${wfMeta.id})` : `unknown (${entry.workflow_id})`;

				activityTasks.set(key, {
					id: sanitizeForMermaid(`act_${key}`),
					name: `${entry.activity_name}${entry.iteration !== undefined ? ` [i${entry.iteration}]` : ''}`,
					start: entry.ts,
					startFracSec: entry.tsFracSec,
					section: sectionName,
					type: 'activity',
					metadata: {
						activity_name: entry.activity_name,
						workflow_id: entry.workflow_id,
						location: entry.location,
						iteration: entry.iteration
					}
				});
			} else if (entry.message === 'activity success' && activityTasks.has(key)) {
				const task = activityTasks.get(key)!;
				task.end = entry.ts;
				task.endFracSec = entry.tsFracSec;
			}
		}

		// Track operation calls (without workflow_id, group separately)
		if (entry.operation_name && !entry.workflow_id) {
			const key = `${entry.operation_name}_${entry.ts.getTime()}`;

			if (entry.message === 'operation call') {
				operationTasks.set(key, {
					id: sanitizeForMermaid(`op_${key}`),
					name: entry.operation_name,
					start: entry.ts,
					startFracSec: entry.tsFracSec,
					section: 'Operations',
					type: 'operation',
					metadata: { operation_name: entry.operation_name }
				});
			} else if (entry.message === 'operation response') {
				// Find the most recent matching operation
				for (const [k, task] of operationTasks.entries()) {
					if (k.startsWith(entry.operation_name) && !task.end) {
						task.end = entry.ts;
						task.endFracSec = entry.tsFracSec;
						break;
					}
				}
			}
		}

		// Track signal dispatching and receiving
		if (entry.signal_name && entry.workflow_id) {
			if (entry.message === 'publishing signal' && entry.signal_id) {
				const wfMeta = workflowMetadata.get(entry.workflow_id);
				const sectionName = wfMeta ? `${wfMeta.name} (${wfMeta.id})` : `unknown (${entry.workflow_id})`;

				// Create send span in sender's workflow section
				const sendKey = `${entry.signal_id}_send`;
				signalTasks.set(sendKey, {
					id: sanitizeForMermaid(`sig_send_${entry.signal_id}`),
					name: `${entry.signal_name} send`,
					start: entry.ts,
					startFracSec: entry.tsFracSec,
					end: entry.ts,
					endFracSec: entry.tsFracSec,
					section: sectionName,
					type: 'signal',
					metadata: { signal_name: entry.signal_name, workflow_id: entry.workflow_id, signal_id: entry.signal_id, is_send: true }
				});

				// Store send time for recv span duration calculation
				signalSendTimes.set(entry.signal_id, {
					ts: entry.ts,
					tsFracSec: entry.tsFracSec,
					signal_name: entry.signal_name
				});
			} else if (entry.message === 'signal received' && entry.signal_id) {
				const wfMeta = workflowMetadata.get(entry.workflow_id);
				const sectionName = wfMeta ? `${wfMeta.name} (${wfMeta.id})` : `unknown (${entry.workflow_id})`;

				// Create recv span in receiver's workflow section
				const sendTime = signalSendTimes.get(entry.signal_id);
				const recvKey = `${entry.signal_id}_recv`;
				signalTasks.set(recvKey, {
					id: sanitizeForMermaid(`sig_recv_${entry.signal_id}`),
					name: `${entry.signal_name} recv`,
					start: entry.ts,
					startFracSec: entry.tsFracSec,
					end: entry.ts,
					endFracSec: entry.tsFracSec,
					section: sectionName,
					type: 'signal',
					metadata: {
						signal_name: entry.signal_name,
						workflow_id: entry.workflow_id,
						signal_id: entry.signal_id,
						is_recv: true,
						send_ts: sendTime?.ts,
						send_frac: sendTime?.tsFracSec
					}
				});
			}
		}
	}

	// Combine all tasks
	const allTasks: Task[] = [
		...workflowExecutionTasks.values(),
		...activityTasks.values(),
		...operationTasks.values(),
		...signalTasks.values()
	];

	// Close any open tasks with the last timestamp
	const lastTime = entries[entries.length - 1].ts;
	for (const task of allTasks) {
		if (!task.end) {
			task.end = lastTime;
		}
	}

	// Sort tasks by start time
	allTasks.sort((a, b) => a.start.getTime() - b.start.getTime());

	// Generate Mermaid Gantt chart
	const lines_out: string[] = [];
	lines_out.push('# Rivet Engine Execution Timeline\n');
	lines_out.push('```mermaid');
	lines_out.push('gantt');
	lines_out.push('    title Rivet Engine Log Timeline');
	lines_out.push('    dateFormat YYYY-MM-DDTHH:mm:ss.SSS');
	lines_out.push('    axisFormat %H:%M:%S');
	lines_out.push('');

	if (splitByWorkflow) {
		// Filter out tasks without workflow_id
		const workflowFilteredTasks = allTasks.filter(task => {
			if (task.type === 'workflow' || task.type === 'activity' || task.type === 'signal') {
				return task.metadata.workflow_id;
			}
			return false;
		});

		// Group tasks by workflow
		const workflowGroups = new Map<string, Task[]>();
		for (const task of workflowFilteredTasks) {
			const wfId = task.metadata.workflow_id;
			if (!workflowGroups.has(wfId)) {
				workflowGroups.set(wfId, []);
			}
			workflowGroups.get(wfId)!.push(task);
		}

		// Output each workflow as a section
		for (const [wfId, tasks] of workflowGroups.entries()) {
			const wfMeta = workflowMetadata.get(wfId);
			const sectionName = wfMeta ? `${wfMeta.name} (${wfId.substring(0, 8)})` : `unknown (${wfId.substring(0, 8)})`;
			lines_out.push(`    section ${sectionName}`);

			for (const task of tasks) {
				const startStr = task.start.toISOString().replace('Z', '').substring(0, 23);
				const endStr = task.end!.toISOString().replace('Z', '').substring(0, 23);

				let duration = 0;
				// For signal receive spans, calculate duration from send time
				if (task.type === 'signal' && task.metadata.is_recv && task.metadata.send_ts) {
					const sendTime = task.metadata.send_ts as Date;
					const sendFrac = task.metadata.send_frac as number;
					const durationMs = task.start.getTime() - sendTime.getTime();
					const durationFracSec = task.startFracSec - sendFrac;
					duration = durationMs + durationFracSec * 1000;
				} else {
					const durationMs = task.end!.getTime() - task.start.getTime();
					const durationFracSec = (task.endFracSec || 0) - task.startFracSec;
					duration = durationMs + durationFracSec * 1000;
				}

				let taskName = '';
				if (task.type === 'workflow') {
					taskName = `wf(${task.name})`;
				} else if (task.type === 'activity') {
					taskName = `act(${task.name})`;
				} else if (task.type === 'signal') {
					taskName = `sig(${task.name})`;
				}

				if (duration > 0.01) {
					taskName = `${taskName} ${formatDuration(duration)}`;
				}

				taskName = taskName.replace(/[,]/g, '');

				let status = '';
				if (task.type === 'activity') {
					status = 'active, ';
				}

				lines_out.push(`    ${taskName} :${status}${task.id}, ${startStr}, ${endStr}`);
			}
			lines_out.push('');
		}
	} else {
		// Output all tasks in single section
		lines_out.push('    section Execution');

		for (const task of allTasks) {
			// Format dates with milliseconds
			const startStr = task.start.toISOString().replace('Z', '').substring(0, 23);
			const endStr = task.end!.toISOString().replace('Z', '').substring(0, 23);

			// Calculate duration with sub-millisecond precision
			let duration = 0;
			// For signal receive spans, calculate duration from send time
			if (task.type === 'signal' && task.metadata.is_recv && task.metadata.send_ts) {
				const sendTime = task.metadata.send_ts as Date;
				const sendFrac = task.metadata.send_frac as number;
				const durationMs = task.start.getTime() - sendTime.getTime();
				const durationFracSec = task.startFracSec - sendFrac;
				duration = durationMs + durationFracSec * 1000;
			} else {
				const durationMs = task.end!.getTime() - task.start.getTime();
				const durationFracSec = (task.endFracSec || 0) - task.startFracSec;
				duration = durationMs + durationFracSec * 1000;
			}

			// Create shortened task name based on type
			let taskName = '';
			if (task.type === 'workflow') {
				const wfId = task.metadata.workflow_id.substring(0, 8);
				taskName = `wf(${task.name} ${wfId})`;
			} else if (task.type === 'activity') {
				const wfId = task.metadata.workflow_id.substring(0, 8);
				taskName = `act(${task.name} ${wfId})`;
			} else if (task.type === 'operation') {
				taskName = `op(${task.name})`;
			} else if (task.type === 'signal') {
				const wfId = task.metadata.workflow_id?.substring(0, 8) || 'unknown';
				taskName = `sig(${task.name} ${wfId})`;
			}

			// Add duration to task name if it's meaningful
			if (duration > 0.01) { // Only show duration if > 10μs
				taskName = `${taskName} ${formatDuration(duration)}`;
			}

			// Remove commas that might break syntax
			taskName = taskName.replace(/[,]/g, '');

			// Determine task status based on type
			let status = '';
			if (task.type === 'activity') {
				status = 'active, ';
			} else if (task.type === 'operation') {
				status = 'crit, ';
			}

			lines_out.push(`    ${taskName} :${status}${task.id}, ${startStr}, ${endStr}`);
		}
		lines_out.push('');
	}

	lines_out.push('```');
	lines_out.push('');
	lines_out.push('## Statistics\n');
	lines_out.push(`- Total log entries: ${entries.length}`);
	lines_out.push(`- Total tasks tracked: ${allTasks.length}`);
	lines_out.push(`- Time range: ${startTime.toISOString()} to ${lastTime.toISOString()}`);
	lines_out.push(`- Duration: ${formatDuration(lastTime.getTime() - startTime.getTime())}`);
	lines_out.push('');
	lines_out.push('### Task Breakdown\n');
	lines_out.push(`- Workflow Executions: ${workflowExecutionTasks.size}`);
	lines_out.push(`- Activities: ${activityTasks.size}`);
	lines_out.push(`- Operations: ${operationTasks.size}`);
	lines_out.push(`- Signals: ${signalTasks.size}`);

	writeFileSync(outputFile, lines_out.join('\n'));
	console.log(`\nGenerated Gantt chart at: ${outputFile}`);
	console.log(`Total tasks: ${allTasks.length}`);
	console.log(`Time range: ${formatDuration(lastTime.getTime() - startTime.getTime())}`);
}

if (require.main === module) {
	main();
}

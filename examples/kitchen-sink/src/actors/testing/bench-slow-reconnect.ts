/**
 * Driver for slowReconnectActor.
 *
 * Warm baseline:
 *   bun bench-slow-reconnect.ts --endpoint http://localhost:6420 --runs=5
 *
 * Cold wake loop:
 *   bun bench-slow-reconnect.ts --endpoint "$RIVET_ENDPOINT" --pool k8s --loop --cold --sleep-ms=5000
 */

import { createClient } from 'rivetkit/client'
import type { registry } from './slow-reconnect-actor'

interface SlowReconnectStep {
	name: string
	durationMs: number
	rowCount: number
}

interface SlowReconnectWorkloadResult {
	name: string
	totalMs: number
	steps: SlowReconnectStep[]
}

interface SlowReconnectResultMessage {
	type: 'slow_reconnect_result'
	trigger: string
	totalMs: number
	results: SlowReconnectWorkloadResult[]
}

interface SlowReconnectErrorMessage {
	type: 'slow_reconnect_error'
	trigger: string
	error: string
}

type ActorMessage = SlowReconnectResultMessage | SlowReconnectErrorMessage | { type: string }

const args = process.argv.slice(2)
const endpoint = readFlagValue('--endpoint') ?? process.env.RIVET_ENDPOINT ?? 'http://localhost:6420'
const poolName = readFlagValue('--pool') ?? process.env.RIVET_POOL
const key = readFlagValue('--key') ?? 'slow-reconnect-rivet-repro'
const runs = Number(readFlagValue('--runs') ?? '1')
const timeoutMs = Number(readFlagValue('--timeout-ms') ?? '120000')
const mode = readFlagValue('--mode') ?? 'executor_connect'
const staggerHandleMs = Number(readFlagValue('--stagger-handle-ms') ?? '0')
const loop = args.includes('--loop')
const cold = args.includes('--cold') || args.includes('--sleep-before-run')
const sleepMs = Number(readFlagValue('--sleep-ms') ?? '5000')
const reconnectDelayMs = Number(readFlagValue('--reconnect-delay-ms') ?? '1000')

if (mode !== 'executor_connect' && mode !== 'repro_reconnect' && mode !== 'client_resume') {
	console.error('Usage: --mode must be executor_connect, repro_reconnect, or client_resume')
	process.exit(1)
}

console.log(`[slow-reconnect] endpoint=${endpoint} pool=${poolName ?? '<default>'} key=${key}`)
console.log(
	`[slow-reconnect] runs=${loop ? '∞' : runs} timeout=${ms(timeoutMs)} mode=${mode} staggerHandleMs=${staggerHandleMs}`,
)
console.log(
	`[slow-reconnect] cold=${cold} sleepMs=${sleepMs} reconnectDelayMs=${reconnectDelayMs}`,
)

const client = createClient<typeof registry>({
	endpoint,
	...(poolName ? { poolName } : {}),
})
let stopping = false

process.on('SIGINT', () => {
	console.log('\n[slow-reconnect] SIGINT, stopping...')
	stopping = true
})

try {
	let index = 1
	while (!stopping && (loop || index <= runs)) {
		try {
			if (cold) {
				await sleepActor(index)
			}
			const result = await runOnce(index)
			printResult(index, result)
		} catch (error) {
			console.error(`[run ${index}] failed:`, error)
			if (!loop) {
				throw error
			}
		}

		index++
		if (loop && !stopping && reconnectDelayMs > 0) {
			await delay(reconnectDelayMs)
		}
	}
} finally {
	await client.dispose()
}

async function sleepActor(index: number): Promise<void> {
	const handle = client.slowReconnectActor.getOrCreate([key])
	const startedAt = performance.now()
	console.log(`\n[run ${index}] sleeping actor...`)
	await (handle as unknown as { sleep: () => Promise<unknown> }).sleep()
	console.log(
		`[run ${index}] sleep action returned in ${ms(performance.now() - startedAt)}; waiting ${ms(sleepMs)} before reconnect`,
	)
	await delay(sleepMs)
}

async function runOnce(index: number): Promise<SlowReconnectResultMessage> {
	const handle = client.slowReconnectActor.getOrCreate([key])
	const startedAt = performance.now()
	console.log(`[run ${index}] opening websocket...`)
	const ws = await handle.webSocket('/', undefined, { skipReadyWait: true })
	if (!ws) {
		throw new Error('slowReconnectActor did not return a WebSocket')
	}
	try {
		await waitForOpen(ws)
		console.log(`[run ${index}] websocket open in ${ms(performance.now() - startedAt)}`)
		const resultPromise = waitForResult(ws, timeoutMs)
		ws.send(JSON.stringify(buildRequest(index)))
		return await resultPromise
	} finally {
		ws.close()
	}
}

function buildRequest(index: number): object {
	if (mode === 'client_resume') {
		return { type: 'client_resume', version: 0 }
	}
	if (mode === 'repro_reconnect') {
		return {
			type: 'repro_reconnect',
			clientId: `slow-reconnect-client-${index}`,
			staggerHandleMs,
		}
	}
	return {
		type: 'executor_connect',
		clientId: `slow-reconnect-client-${index}`,
		executorType: 'local-client',
		capabilities: {},
	}
}

function waitForResult(ws: WebSocket, timeoutMs: number): Promise<SlowReconnectResultMessage> {
	return new Promise((resolve, reject) => {
		const timeout = setTimeout(() => {
			reject(new Error(`Timed out after ${timeoutMs}ms waiting for slowReconnectActor result`))
			try {
				ws.close()
			} catch {}
		}, timeoutMs)

		const cleanup = () => clearTimeout(timeout)

		ws.addEventListener('message', (event: MessageEvent) => {
			const data = typeof event.data === 'string' ? event.data : ''
			if (data === 'pong') {
				return
			}
			let message: ActorMessage
			try {
				message = JSON.parse(data) as ActorMessage
			} catch {
				console.log(`[slow-reconnect] <<< ${data.slice(0, 200)}`)
				return
			}
			if (message.type === 'executor_connected') {
				console.log('[slow-reconnect] <<< executor_connected')
				return
			}
			if (message.type === 'slow_reconnect_error') {
				cleanup()
				reject(new Error((message as SlowReconnectErrorMessage).error))
				return
			}
			if (message.type === 'slow_reconnect_result') {
				cleanup()
				resolve(message as SlowReconnectResultMessage)
			}
		})

		ws.addEventListener('close', (event: CloseEvent) => {
			cleanup()
			reject(
				new Error(
					`WebSocket closed before result: code=${event.code} reason=${event.reason || '<empty>'}`,
				),
			)
		})
		ws.addEventListener('error', () => {
			cleanup()
			reject(new Error('WebSocket failed while waiting for result'))
		})
	})
}

async function waitForOpen(ws: WebSocket): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) {
		return
	}
	await new Promise<void>((resolve, reject) => {
		ws.addEventListener('open', () => resolve(), { once: true })
		ws.addEventListener('error', () => reject(new Error('WebSocket failed to open')), {
			once: true,
		})
		ws.addEventListener('close', () => reject(new Error('WebSocket closed before open')), {
			once: true,
		})
	})
}

function printResult(index: number, message: SlowReconnectResultMessage): void {
	console.log(`\n[run ${index}] trigger=${message.trigger} total=${ms(message.totalMs)}`)
	for (const workload of message.results) {
		console.log(`  ${workload.name.padEnd(28)} total=${ms(workload.totalMs)}`)
		for (const step of workload.steps) {
			console.log(
				`    ${step.name.padEnd(36)} ${ms(step.durationMs).padStart(8)} rows=${step.rowCount}`,
			)
		}
	}
}

function readFlagValue(flag: string): string | undefined {
	const prefix = `${flag}=`
	const equalsValue = args.find((arg) => arg.startsWith(prefix))
	if (equalsValue) {
		return equalsValue.slice(prefix.length)
	}
	const index = args.indexOf(flag)
	if (index === -1) {
		return undefined
	}
	return args[index + 1]
}

function ms(value: number): string {
	return `${Math.round(value)}ms`
}

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, Math.max(0, ms)))
}

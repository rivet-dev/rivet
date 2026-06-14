import type { FaqItem } from './types';

// FAQ content for the Rivet Actors marketing page. Rendered statically on
// pages/actors.astro and emitted as FAQPage JSON-LD from the same page.
export const actorsFaqs: FaqItem[] = [
	{
		question: 'What is a Rivet Actor?',
		answerHtml:
			'A Rivet Actor is a long-lived unit of compute that keeps its own state between requests. It is like a serverless function with memory and no timeouts. The common pattern is one actor per agent, per session, or per user, with state, storage, and networking included. See the <a href="https://rivet.dev/docs/actors/">Rivet Actors docs</a> for details.',
	},
	{
		question: 'How are Rivet Actors different from serverless functions?',
		answerHtml:
			'Serverless functions are stateless and short-lived, so every invocation re-fetches data from a database and long-running work hits timeouts. A Rivet Actor remembers state between requests, holds WebSocket connections, and runs as long as it has work to do. It still sleeps when idle, so you keep the scale-to-zero economics of serverless.',
	},
	{
		question: 'Do I need a separate database for actor state?',
		answerHtml:
			'No. State lives on the same machine as your compute, so reads and writes are fast with no database round trips. Use <a href="https://rivet.dev/docs/actors/state/">in-memory state</a> for small values and each actor\'s built-in SQLite database for large, relational, or queryable data. State is persisted, so it survives restarts, crashes, and deployments. You can still connect to an external database when you need one.',
	},
	{
		question: 'How do Rivet Actors compare to Cloudflare Durable Objects?',
		answerHtml:
			'Cloudflare Durable Objects provide stateful serverless computing tied to the Cloudflare platform. Rivet Actors give you the same actor model capabilities as open source software that works with your existing infrastructure, so you can deploy on Kubernetes, AWS, a VPS, or Rivet Cloud without vendor lock-in. See the full <a href="https://rivet.dev/compare/rivet-vs-cloudflare-durable-objects/">Rivet vs Cloudflare Durable Objects comparison</a>.',
	},
	{
		question: 'What happens when an actor is idle?',
		answerHtml:
			'Actors automatically sleep after a period of inactivity to free up resources. Their state is persisted, and they wake on demand when a request, message, or scheduled alarm arrives. Sleeping actors consume no compute, which makes Rivet Actors cost-efficient for bursty workloads. See the <a href="https://rivet.dev/docs/actors/lifecycle/">lifecycle docs</a> for how sleeping works.',
	},
	{
		question: 'How do Rivet Actors scale?',
		answerHtml:
			'The core pattern is one actor per entity in your system: a user, a document, a chat room, or an agent. Each actor is small and independent, so the system scales from zero to millions of concurrent actors automatically, with instant scaling and no cold starts.',
	},
	{
		question: 'Do Rivet Actors support WebSockets and realtime updates?',
		answerHtml:
			'Yes. Realtime is built in. Clients call actions to send data to an actor, and the actor pushes updates back over WebSockets with <a href="https://rivet.dev/docs/actors/events/">events</a>, either to specific connections or broadcast to all of them. There is no external pub/sub system or polling to manage.',
	},
	{
		question: 'What languages and frameworks can I use?',
		answerHtml:
			'Rivet Actors are written in TypeScript or JavaScript and run on Node.js, Bun, and Deno, with a Rust SDK available in preview. Quickstarts cover <a href="https://rivet.dev/docs/actors/quickstart/">backend, React, Next.js, and Rust</a> apps. Client libraries are available for JavaScript, React, and Swift.',
	},
];

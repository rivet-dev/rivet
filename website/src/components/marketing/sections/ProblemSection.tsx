'use client';

import { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Database, Cpu, Workflow, Clock, Wifi, Zap, Bot, Users, Timer, Boxes, Radio, MessageSquare, ArrowRight } from 'lucide-react';
import { codeToHtml } from 'shiki';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '../typography';
import heroTheme from '@/lib/agent-os-hero-code-theme';

const RivetIcon = ({ className }: { className?: string }) => (
  <svg width="16" height="16" viewBox="0 0 176 173" className={className}>
    <g transform="translate(-32928.8,-28118.2)">
      <g transform="matrix(0.941176,0,0,0.925134,2119.4,2323.67)">
        <g clipPath="url(#_clip1)">
          <g transform="matrix(1.0625,0,0,1.08092,32936.6,27881.1)">
            <path d="M164.529,52.792L164.529,120.844C164.529,145.347 144.635,165.241 120.132,165.241L52.08,165.241C27.577,165.241 7.683,145.347 7.683,120.844L7.683,52.792C7.683,28.289 27.577,8.395 52.08,8.395L120.132,8.395C144.635,8.395 164.529,28.289 164.529,52.792Z" style={{ fill: 'none', stroke: 'currentColor', strokeWidth: '15.18px' }} />
          </g>
          <g transform="matrix(1.0625,0,0,1.08092,32737,27881.7)">
            <path d="M164.529,52.792L164.529,120.844C164.529,145.347 144.635,165.241 120.132,165.241L52.08,165.241C27.577,165.241 7.683,145.347 7.683,120.844L7.683,52.792C7.683,28.289 27.577,8.395 52.08,8.395L120.132,8.395C144.635,8.395 164.529,28.289 164.529,52.792Z" style={{ fill: 'none', stroke: 'currentColor', strokeWidth: '15.18px' }} />
          </g>
        </g>
      </g>
    </g>
    <g transform="translate(-32928.8,-28118.2)">
      <g transform="matrix(0.941176,0,0,0.925134,2119.4,2323.67)">
        <g clipPath="url(#_clip1)">
          <g transform="matrix(1.0625,0,0,1.08092,-2251.86,-2261.21)">
            <g transform="translate(32930.7,27886.2)">
              <path d="M104.323,87.121C104.584,85.628 105.665,84.411 107.117,83.977C108.568,83.542 110.14,83.965 111.178,85.069C118.49,92.847 131.296,106.469 138.034,113.637C138.984,114.647 139.343,116.076 138.983,117.415C138.623,118.754 137.595,119.811 136.267,120.208C127.471,122.841 111.466,127.633 102.67,130.266C101.342,130.664 99.903,130.345 98.867,129.425C97.83,128.504 97.344,127.112 97.582,125.747C99.274,116.055 102.488,97.637 104.323,87.121Z" style={{ fill: 'currentColor' }} />
            </g>
            <g transform="translate(32930.7,27886.2)">
              <path d="M69.264,88.242L79.739,106.385C82.629,111.392 80.912,117.803 75.905,120.694L57.762,131.168C52.755,134.059 46.344,132.341 43.453,127.335L32.979,109.192C30.088,104.185 31.806,97.774 36.813,94.883L54.956,84.408C59.962,81.518 66.374,83.236 69.264,88.242Z" style={{ fill: 'currentColor' }} />
            </g>
            <g transform="translate(32930.7,27886.2)">
              <path d="M86.541,79.464C98.111,79.464 107.49,70.084 107.49,58.514C107.49,46.944 98.111,37.565 86.541,37.565C74.971,37.565 65.591,46.944 65.591,58.514C65.591,70.084 74.971,79.464 86.541,79.464Z" style={{ fill: 'currentColor' }} />
            </g>
          </g>
        </g>
      </g>
    </g>
  </svg>
);

// Client-side shiki highlighting hook
const useHighlightedCode = (code: string) => {
  const [html, setHtml] = useState<string>('');
  const cache = useRef<Record<string, string>>({});

  useEffect(() => {
    if (cache.current[code]) {
      setHtml(cache.current[code]);
      return;
    }

    codeToHtml(code, {
      lang: 'typescript',
      theme: heroTheme,
    }).then((result) => {
      cache.current[code] = result;
      setHtml(result);
    });
  }, [code]);

  return html;
};

interface UseCaseConfig {
  title: string;
  description: string;
  features: { icon: typeof Cpu; label: string; detail: string; href: string }[];
  serverCode: string;
  clientCode: string;
}

const useCases: Record<string, UseCaseConfig> = {
  'AI Agent': {
    title: 'AI Agent',
    description: 'Each agent runs as its own actor with persistent context, memory, and the ability to schedule tool calls.',
    features: [
      { icon: Cpu, label: 'In-memory state', detail: 'Context', href: '/docs/actors/state' },
      { icon: Database, label: 'SQLite or BYO database persistence', detail: 'Memory', href: '/docs/actors/state' },
      { icon: Clock, label: 'Scheduling', detail: 'Tool calls', href: '/docs/actors/schedule' },
    ],
    serverCode: `const agent = actor({
  // In-memory, persisted state for the actor
  state: { messages: [] },

  // Long-running actor process
  run: async (c) => {
    // Process incoming messages from the queue
    for await (const msg of c.queue.iter()) {
      c.state.messages.push({ role: "user", content: msg.body.text });
      const response = streamText({ model: openai("gpt-5"), messages: c.state.messages });

      // Stream realtime events to all connected clients
      for await (const delta of response.textStream) {
        c.broadcast("token", delta);
      }

      c.state.messages.push({ role: "assistant", content: await response.text });
    }
  },
});`,
    clientCode: `const agent = client.agent.getOrCreate("agent-123").connect();
agent.on("token", delta => process.stdout.write(delta));
await agent.queue.send("hello!");`,
  },
  'Workflows': {
    title: 'Workflows',
    description: 'Multi-step operations with automatic retries, scheduling, and durable state across steps.',
    features: [
      { icon: Workflow, label: 'Workflows', detail: 'Steps', href: '/docs/actors/workflows' },
      { icon: Clock, label: 'Scheduling', detail: 'Retry', href: '/docs/actors/schedule' },
      { icon: Database, label: 'SQLite or BYO database persistence', detail: 'State', href: '/docs/actors/state' },
    ],
    serverCode: `// One actor per workflow run
const workflow = actor({
  // Progress survives crashes
  state: { step: 0, results: {} },
  actions: {
    run: async (c) => {
      c.state.results.data = await fetchData();
      c.state.step = 1;
      // Automatically retries on failure
      c.state.results.processed = await transform(
        c.state.results.data
      );
      c.state.step = 2;
    },
  },
});`,
    clientCode: `const job = client.workflow.create();
await job.run();`,
  },
  'Collab Docs': {
    title: 'Collaborate Document',
    description: 'Real-time collaborative editing where each document is an actor broadcasting changes to all connected users.',
    features: [
      { icon: Cpu, label: 'In-memory state', detail: 'Document', href: '/docs/actors/state' },
      { icon: Wifi, label: 'WebSockets', detail: 'Sync', href: '/docs/actors/events' },
      { icon: Zap, label: 'Runs indefinitely', detail: 'Always on', href: '/docs/actors/lifecycle' },
    ],
    serverCode: `// One actor per document
const document = actor({
  state: { content: "", version: 0 },
  actions: {
    edit: (c, patch) => {
      c.state.content = applyPatch(
        c.state.content, patch
      );
      c.state.version++;
      // Send realtime update to all clients
      c.broadcast("update", c.state);
    },
  },
});`,
    clientCode: `const doc = client.document.get("doc-789");
await doc.edit({ insert: "Hello", pos: 0 });
doc.on("update", (state) => render(state));`,
  },
  'Per-Tenant Database': {
    title: 'Per-Tenant DB',
    description: 'One actor per tenant with low-latency in-memory reads and durable tenant data persistence.',
    features: [
      { icon: Cpu, label: 'In-memory state', detail: 'Hot reads', href: '/docs/actors/state' },
      { icon: Database, label: 'SQLite or BYO database persistence', detail: 'Tenant data', href: '/docs/actors/state' },
      { icon: Zap, label: 'Sleeps when idle', detail: 'Cost efficient', href: '/docs/actors/lifecycle' },
    ],
    serverCode: `// One actor per tenant
const tenantDb = actor({
  state: { users: {}, settings: {} },
  actions: {
    upsertUser: (c, user) => {
      c.state.users[user.id] = user;
      return c.state.users[user.id];
    },
    getUser: (c, userId) => c.state.users[userId] ?? null,
  },
});`,
    clientCode: `const tenant = client.tenantDb.get("tenant-123");
await tenant.upsertUser({ id: "u1", name: "Avery" });
const user = await tenant.getUser("u1");`,
  },
  'Sandbox Orchestration': {
    title: 'Sandbox Orchestration',
    description: 'Coordinate sandbox sessions, queue work, and schedule cleanup in one long-lived actor per workspace.',
    features: [
      { icon: Cpu, label: 'In-memory state', detail: 'Live sessions', href: '/docs/actors/state' },
      { icon: Database, label: 'Queue messages', detail: 'Jobs', href: '/docs/actors/queues' },
      { icon: Clock, label: 'Scheduling', detail: 'Timeouts', href: '/docs/actors/schedule' },
    ],
    serverCode: `// One actor per sandbox workspace
const sandbox = actor({
  state: { sessions: {}, pendingRuns: [] },
  actions: {
    enqueueRun: (c, run) => {
      c.state.pendingRuns.push(run);
      c.schedule.after(0, "processQueue");
    },
    processQueue: async (c) => {
      const run = c.state.pendingRuns.shift();
      if (!run) return;
      const result = await executeInSandbox(run);
      c.broadcast("runComplete", result);
    },
  },
});`,
    clientCode: `const sandbox = client.sandbox.get("workspace-123");
await sandbox.enqueueRun({ sessionId: "abc", command: "pnpm test" });
sandbox.on("runComplete", (result) => render(result));`,
  },
  'Chat': {
    title: 'Chat',
    description: 'One actor per room or conversation with in-memory state, persistent history, and realtime delivery.',
    features: [
      { icon: Cpu, label: 'In-memory state', detail: 'Room state', href: '/docs/actors/state' },
      { icon: Database, label: 'SQLite or BYO database persistence', detail: 'History', href: '/docs/actors/state' },
      { icon: Wifi, label: 'WebSockets', detail: 'Realtime', href: '/docs/actors/events' },
    ],
    serverCode: `// One actor per chat room
const chatRoom = actor({
  state: { messages: [] },
  actions: {
    send: (c, text) => {
      const msg = { text, sentAt: Date.now() };
      c.state.messages.push(msg);
      c.broadcast("message", msg);
    },
    history: (c) => c.state.messages,
  },
});`,
    clientCode: `const room = client.chatRoom.get("room-123");
await room.send("Hello everyone");
room.on("message", (msg) => renderMessage(msg));`,
  },
};

type UseCaseKey = keyof typeof useCases;

const useCaseOrder: UseCaseKey[] = [
  'AI Agent',
  'Sandbox Orchestration',
  'Workflows',
  'Collab Docs',
  'Chat',
  'Per-Tenant Database',
];

const useCaseTabLabels: Record<UseCaseKey, string> = {
  'AI Agent': 'AI Agent',
  'Sandbox Orchestration': 'Sandboxes',
  'Workflows': 'Workflows',
  'Collab Docs': 'Multiplayer',
  'Per-Tenant Database': 'Per-Tenant DB',
  'Chat': 'Chat',
};

const useCaseIcons: Record<string, typeof Bot> = {
  'AI Agent': Bot,
  'Workflows': Timer,
  'Collab Docs': Users,
  'Per-Tenant Database': Database,
  'Sandbox Orchestration': Boxes,
  'Chat': MessageSquare,
};

const HighlightedCode = ({ code, title }: { code: string; title: string }) => {
  const html = useHighlightedCode(code);

  return (
    <div>
      <div className='px-4 py-2 border-b border-zinc-200 text-xs text-zinc-500 font-mono'>
        {title}
      </div>
      {!html ? (
        <pre className='p-4 font-mono text-xs md:text-sm leading-6 text-zinc-600 overflow-x-auto'>
          <code>{code}</code>
        </pre>
      ) : (
        <div
          className='p-4 text-xs md:text-sm leading-6 overflow-x-auto [&_pre]:!bg-transparent [&_pre]:!m-0 [&_pre]:!p-0 [&_code]:!bg-transparent'
          dangerouslySetInnerHTML={{ __html: html }}
        />
      )}
    </div>
  );
};

// Stacked code snippets that live inside the section's ink plate.
const UseCaseCode = ({ config }: { config: UseCaseConfig }) => (
  <div>
    <HighlightedCode code={config.serverCode} title='backend.ts' />
    <div className='border-t border-zinc-200'>
      <HighlightedCode code={config.clientCode} title='client.ts' />
    </div>
  </div>
);

// The porcelain caption column that sits beside the ink plate.
const UseCaseDetails = ({ config }: { config: UseCaseConfig }) => (
  <div className='flex flex-col gap-6'>
    <div className='flex items-center gap-3'>
      <RivetIcon className='text-olive' />
      <div className='flex items-center gap-2'>
        <span className='text-sm font-medium uppercase tracking-wider text-ink'>Rivet Actor</span>
        <span className='text-sm font-medium uppercase tracking-wider text-ink-faint'>/ {config.title}</span>
      </div>
    </div>

    <p className='text-sm leading-relaxed text-ink-soft'>
      {config.description}
    </p>

    <div className='flex flex-col gap-3'>
      {config.features.map((feature, idx) => {
        const Icon = feature.icon;
        return (
          <a
            key={idx}
            href={feature.href}
            className='group w-fit flex items-center gap-3 text-ink-soft transition-colors duration-200 hover:text-pine'
          >
            <Icon className='h-4 w-4 flex-shrink-0 text-ink-faint transition-colors duration-200 group-hover:text-pine' />
            <span className='text-sm'>{feature.label}</span>
            <span className='text-sm text-ink-faint transition-colors duration-200 group-hover:text-pine/80'>({feature.detail})</span>
            <ArrowRight className='h-3.5 w-3.5 text-pine opacity-0 -translate-x-1 transition-all duration-200 group-hover:opacity-100 group-hover:translate-x-0' />
          </a>
        );
      })}
    </div>
  </div>
);

export const ProblemSection = () => {
  const [activeUseCase, setActiveUseCase] = useState<UseCaseKey>('AI Agent');
  const [showScrollHint, setShowScrollHint] = useState(false);
  const tabsScrollRef = useRef<HTMLDivElement>(null);
  const config = useCases[activeUseCase];

  useEffect(() => {
    const tabsEl = tabsScrollRef.current;
    if (!tabsEl) return;

    const updateScrollHint = () => {
      const hasOverflow = tabsEl.scrollWidth > tabsEl.clientWidth + 1;
      const atRightEdge = tabsEl.scrollLeft + tabsEl.clientWidth >= tabsEl.scrollWidth - 1;
      setShowScrollHint(hasOverflow && !atRightEdge);
    };

    updateScrollHint();

    tabsEl.addEventListener('scroll', updateScrollHint, { passive: true });
    window.addEventListener('resize', updateScrollHint);

    return () => {
      tabsEl.removeEventListener('scroll', updateScrollHint);
      window.removeEventListener('resize', updateScrollHint);
    };
  }, []);

  return (
    <section id='problem' className='relative px-4 lg:px-6 py-16 md:py-32'>
      <div className='mx-auto w-full max-w-7xl'>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='mb-12'
        >
          <h2 className={SECTION_H2_CLASS}>
            Actors in action.
          </h2>
          <p className={SUBTITLE_CLASS}>
            One primitive that adapts to agents, workflows, collaboration, and more.
          </p>
        </motion.div>

        {/* Editorial split: the ink plate carries the tabs and code; the
            porcelain caption column carries the description and features. */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.05 }}
          className='grid grid-cols-1 items-start gap-10 lg:grid-cols-[minmax(0,1.55fr)_minmax(0,1fr)] lg:gap-14'
        >
          <div className='order-2 overflow-hidden rounded-xl border border-zinc-200 bg-zinc-50 lg:order-1'>
            {/* File-tab strip */}
            <div className='relative'>
              <div
                ref={tabsScrollRef}
                className='flex w-full overflow-x-auto scrollbar-hide border-b border-zinc-200'
              >
                {useCaseOrder.map((useCase) => {
                  const Icon = useCaseIcons[useCase];
                  const active = activeUseCase === useCase;
                  return (
                    <button
                      key={useCase}
                      type="button"
                      onClick={() => setActiveUseCase(useCase)}
                      className={`-mb-px flex flex-none items-center justify-center gap-2 whitespace-nowrap border-b-2 px-4 py-3 font-mono text-xs transition-colors sm:flex-1 ${
                        active
                          ? 'border-pine text-ink'
                          : 'border-transparent text-ink-soft hover:text-ink'
                      }`}
                    >
                      {Icon && <Icon className='h-3.5 w-3.5' />}
                      {useCaseTabLabels[useCase]}
                    </button>
                  );
                })}
              </div>
              <div
                className={`pointer-events-none absolute right-0 top-0 bottom-0 flex items-center pr-2 transition-opacity duration-200 ${
                  showScrollHint ? 'opacity-100' : 'opacity-0'
                }`}
              >
                <div className='absolute inset-y-0 right-0 w-14 bg-gradient-to-l from-zinc-50 to-transparent' />
                <span className='relative inline-flex h-6 w-6 items-center justify-center rounded-full border border-zinc-200 bg-zinc-50'>
                  <ArrowRight className='h-3.5 w-3.5 text-ink-soft' />
                </span>
              </div>
            </div>

            <AnimatePresence mode='wait'>
              <motion.div
                key={activeUseCase}
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -8 }}
                transition={{ duration: 0.2 }}
              >
                <UseCaseCode config={config} />
              </motion.div>
            </AnimatePresence>
          </div>

          <div className='order-1 lg:order-2'>
            <AnimatePresence mode='wait'>
              <motion.div
                key={activeUseCase}
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -8 }}
                transition={{ duration: 0.2 }}
              >
                <UseCaseDetails config={config} />
              </motion.div>
            </AnimatePresence>
          </div>
        </motion.div>
      </div>
    </section>
  );
};

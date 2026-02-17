'use client';

import { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Database, Cpu, Workflow, Clock, Wifi, Zap, Bot, BrainCircuit, Users, Timer, UserCircle, Radio } from 'lucide-react';
import { codeToHtml } from 'shiki';

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
      theme: 'ayu-dark',
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
  features: { icon: typeof Cpu; label: string; detail: string }[];
  serverCode: string;
  clientCode: string;
}

const useCases: Record<string, UseCaseConfig> = {
  'AI Agent': {
    title: 'AI Agent',
    description: 'Each agent runs as its own actor with persistent context, memory, and the ability to schedule tool calls.',
    features: [
      { icon: Cpu, label: 'In-memory state', detail: 'Context' },
      { icon: Database, label: 'SQLite or JSON persistence', detail: 'Memory' },
      { icon: Clock, label: 'Scheduling', detail: 'Tool calls' },
    ],
    serverCode: `// One actor per agent
const agent = actor({
  // State is persisted automatically
  state: { messages: [], memory: {} },
  actions: {
    chat: (c, message) => {
      c.state.messages.push(message);
      const response = await c.llm.chat(c.state);
      c.state.memory = response.memory;
      return response.text;
    },
  },
});`,
    clientCode: `const agent = client.agent.get("agent-123");
const reply = await agent.chat("Hello!");`,
  },
  'Agent Memory': {
    title: 'Agent Memory',
    description: 'Persistent per-agent memory that sleeps when idle and wakes instantly when needed.',
    features: [
      { icon: Cpu, label: 'In-memory state', detail: 'Context' },
      { icon: Database, label: 'SQLite or JSON persistence', detail: 'History' },
      { icon: Zap, label: 'Sleeps when idle', detail: 'Cost efficient' },
    ],
    serverCode: `// One actor per agent's memory
const memory = actor({
  // Persisted across restarts
  state: { entries: [], summary: "" },
  actions: {
    store: (c, entry) => {
      c.state.entries.push(entry);
    },
    recall: (c, query) => {
      return c.state.entries.filter(
        e => e.tags.includes(query)
      );
    },
  },
});`,
    clientCode: `const mem = client.memory.get("user-456");
await mem.store({ text: "likes coffee", tags: ["prefs"] });
const results = await mem.recall("prefs");`,
  },
  'Workflows': {
    title: 'Workflows',
    description: 'Multi-step operations with automatic retries, scheduling, and durable state across steps.',
    features: [
      { icon: Workflow, label: 'Workflows', detail: 'Steps' },
      { icon: Clock, label: 'Scheduling', detail: 'Retry' },
      { icon: Database, label: 'SQLite or JSON persistence', detail: 'State' },
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
    title: 'Collab Docs',
    description: 'Real-time collaborative editing where each document is an actor broadcasting changes to all connected users.',
    features: [
      { icon: Cpu, label: 'In-memory state', detail: 'Document' },
      { icon: Wifi, label: 'WebSockets', detail: 'Sync' },
      { icon: Zap, label: 'Runs indefinitely', detail: 'Always on' },
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
  'Realtime Sync': {
    title: 'Realtime Sync',
    description: 'Live state synchronization across clients with WebSocket connections and indefinite uptime.',
    features: [
      { icon: Cpu, label: 'In-memory state', detail: 'State' },
      { icon: Wifi, label: 'WebSockets', detail: 'Events' },
      { icon: Zap, label: 'Runs indefinitely', detail: 'Always on' },
    ],
    serverCode: `// One actor per shared resource
const sync = actor({
  state: { data: {}, clients: [] },
  actions: {
    update: (c, key, value) => {
      c.state.data[key] = value;
      // Broadcast changes to all connections
      c.broadcast("sync", { key, value });
    },
  },
});`,
    clientCode: `const room = client.sync.get("room-101");
await room.update("cursor", { x: 10, y: 20 });
room.on("sync", (data) => updateUI(data));`,
  },
  'Session Store': {
    title: 'Session Store',
    description: 'Per-user session actors that persist auth state and user data, sleeping when idle to save resources.',
    features: [
      { icon: Database, label: 'SQLite or JSON persistence', detail: 'User data' },
      { icon: Cpu, label: 'In-memory state', detail: 'Auth state' },
      { icon: Zap, label: 'Sleeps when idle', detail: 'Cost efficient' },
    ],
    serverCode: `// One actor per user session
const session = actor({
  // Sleeps when idle, wakes instantly
  state: { user: null, prefs: {} },
  actions: {
    login: (c, credentials) => {
      c.state.user = authenticate(credentials);
      return { token: c.state.user.token };
    },
    getPrefs: (c) => c.state.prefs,
  },
});`,
    clientCode: `const session = client.session.get("user-123");
const { token } = await session.login(credentials);
const prefs = await session.getPrefs();`,
  },
};

type UseCaseKey = keyof typeof useCases;

const useCaseOrder: UseCaseKey[] = [
  'AI Agent',
  'Agent Memory',
  'Workflows',
  'Collab Docs',
  'Realtime Sync',
  'Session Store',
];

const useCaseIcons: Record<string, typeof Bot> = {
  'AI Agent': Bot,
  'Agent Memory': BrainCircuit,
  'Workflows': Timer,
  'Collab Docs': Users,
  'Realtime Sync': Radio,
  'Session Store': UserCircle,
};

const HighlightedCode = ({ code, title }: { code: string; title: string }) => {
  const html = useHighlightedCode(code);

  return (
    <div>
      <div className='px-4 py-2 border-b border-white/[0.12] text-xs text-zinc-500 font-mono'>
        {title}
      </div>
      {!html ? (
        <pre className='p-4 font-mono text-xs md:text-sm leading-6 text-zinc-400 overflow-x-auto'>
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

const UseCaseContent = ({ config }: { config: UseCaseConfig }) => {
  return (
    <div className='grid grid-cols-1 lg:grid-cols-2 gap-6 lg:gap-10 items-start'>
      {/* Left: Stacked code snippets */}
      <div className='flex flex-col gap-3'>
        <div className='rounded-lg border border-white/[0.12] bg-white/[0.03] overflow-hidden'>
          <HighlightedCode code={config.serverCode} title='backend.ts' />
        </div>
        <div className='rounded-lg border border-white/[0.12] bg-white/[0.03] overflow-hidden'>
          <HighlightedCode code={config.clientCode} title='client.ts' />
        </div>
      </div>

      {/* Right: Description + features */}
      <div className='flex flex-col gap-6'>
        <div className='flex items-center gap-3'>
          <RivetIcon className='text-zinc-500' />
          <div className='flex items-center gap-2'>
            <span className='text-sm font-medium uppercase tracking-wider text-white'>Rivet Actor</span>
            <span className='text-sm font-medium uppercase tracking-wider text-zinc-500'>/ {config.title}</span>
          </div>
        </div>

        <p className='text-sm leading-relaxed text-zinc-400'>
          {config.description}
        </p>

        <div className='flex flex-col gap-3'>
          {config.features.map((feature, idx) => {
            const Icon = feature.icon;
            return (
              <div key={idx} className='flex items-center gap-3'>
                <Icon className='h-4 w-4 text-zinc-500 flex-shrink-0' />
                <span className='text-sm text-zinc-300'>{feature.label}</span>
                <span className='text-sm text-zinc-600'>({feature.detail})</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};

export const ProblemSection = () => {
  const [activeUseCase, setActiveUseCase] = useState<UseCaseKey>('AI Agent');
  const config = useCases[activeUseCase];

  return (
    <section id='problem' className='relative border-b border-white/10 px-4 lg:px-6 py-20 lg:py-32'>
      <div className='mx-auto w-full max-w-7xl'>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='mb-12'
        >
          <h2 className='mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl'>
            See it in action.
          </h2>
          <p className='text-base leading-relaxed text-zinc-500'>
            One primitive that adapts to agents, workflows, collaboration, and more.
          </p>
        </motion.div>

        {/* Segmented control */}
        <div className='mb-10'>
          <div className='flex w-full overflow-x-auto scrollbar-hide rounded-lg border border-white/[0.12] bg-white/[0.03] p-1 cursor-pointer'>
            {useCaseOrder.map((useCase, idx) => {
              const Icon = useCaseIcons[useCase];
              return (
                <div key={useCase} className='flex flex-1 min-w-0 items-center'>
                  {idx > 0 && (
                    <div className='w-px self-stretch my-1.5 mx-1 bg-white/[0.12] flex-shrink-0' />
                  )}
                  <button
                    type="button"
                    onClick={() => setActiveUseCase(useCase)}
                    className={`whitespace-nowrap rounded-md px-4 py-2.5 text-xs md:text-sm transition-all flex items-center justify-center gap-2 flex-1 ${
                      activeUseCase === useCase
                        ? 'bg-[#FF4500]/15 text-[#FF4500] border border-[#FF4500]/20'
                        : 'text-zinc-500 hover:text-zinc-300 border border-transparent'
                    }`}
                  >
                    {Icon && <Icon className='h-3.5 w-3.5' />}
                    {useCase}
                  </button>
                </div>
              );
            })}
          </div>
        </div>

        <div className='h-[600px] md:h-[520px] lg:h-[480px] overflow-hidden'>
          <AnimatePresence mode='wait'>
            <motion.div
              key={activeUseCase}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.2 }}
              className='h-full'
            >
              <UseCaseContent config={config} />
            </motion.div>
          </AnimatePresence>
        </div>
      </div>
    </section>
  );
};

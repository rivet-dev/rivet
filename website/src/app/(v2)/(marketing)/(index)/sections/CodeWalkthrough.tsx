'use client';

import { useEffect, useState, useRef } from 'react';
import { motion } from 'framer-motion';

export const CodeWalkthrough = () => {
  const [activeStep, setActiveStep] = useState(0);
  const observerRefs = useRef([]);

  const steps = [
    {
      title: 'Define the Actor',
      description:
        'Start by writing an actor template. This will be used to create actors with their own isolated state.',
      lines: [0, 2, 14]
    },
    {
      title: 'Declare State',
      description:
        'Define the shape of your data. This state object is automatically persisted to disk and loaded into memory when the actor wakes up. No database queries needed.',
      lines: [3]
    },
    {
      title: 'Write Actions',
      description:
        "Actions are logic that runs on your actor. They run directly in the actor's memory space with zero network latency to access the state.",
      lines: [5, 6, 12, 13]
    },
    {
      title: 'Mutate State Directly',
      description:
        'Just modify the state variable. Rivet detects the changes and handles the persistence and replication for you.',
      lines: [7, 8]
    },
    {
      title: 'Broadcast Realtime Events',
      description:
        "Push updates to all connected clients instantly using WebSockets. It's built right into the context object.",
      lines: [10]
    }
  ];

  const codeLines = [
    `import { actor } from "rivetkit";`,
    ``,
    `export const chatRoom = actor({`,
    `  state: { messages: [] },`,
    ``,
    `  actions: {`,
    `    postMessage: (c, text) => {`,
    `      const msg = { text, at: Date.now() };`,
    `      c.state.messages.push(msg);`,
    ``,
    `      c.broadcast("newMessage", msg);`,
    `      return "sent";`,
    `    }`,
    `  }`,
    `});`
  ];

  useEffect(() => {
    const options = {
      root: null,
      rootMargin: '-40% 0px -40% 0px',
      threshold: 0
    };

    const observer = new IntersectionObserver(entries => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          const index = Number(entry.target.dataset.index);
          setActiveStep(index);
        }
      });
    }, options);

    observerRefs.current.forEach(ref => {
      if (ref) observer.observe(ref);
    });

    return () => observer.disconnect();
  }, []);

  return (
    <section className='relative border-t border-white/10 bg-black py-32'>
      <div className='mx-auto max-w-7xl px-6'>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='mb-16'
        >
          <h2 className='mb-6 text-3xl font-medium tracking-tight text-white md:text-5xl'>How it works</h2>
          <p className='max-w-2xl text-lg leading-relaxed text-zinc-400'>
            Rivet makes backend development feel like frontend development. Define your state, write your
            logic, and let Rivet handle the rest.
          </p>
        </motion.div>

        <div className='grid grid-cols-1 gap-16 lg:grid-cols-2'>
          {/* Sticky Code Block */}
          <div className='relative hidden lg:block'>
            <div className='sticky top-32'>
              <div className='relative overflow-hidden rounded-xl border border-white/10 bg-zinc-900/50 shadow-2xl backdrop-blur-xl transition-all duration-500'>
                {/* Top Shine Highlight */}
                <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/30 to-transparent' />
                <div className='flex items-center gap-2 border-b border-white/5 bg-white/5 px-4 py-3'>
                  <div className='flex gap-1.5'>
                    <div className='h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20' />
                    <div className='h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20' />
                    <div className='h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20' />
                  </div>
                  <span className='ml-2 font-mono text-xs text-zinc-500'>chat-room.ts</span>
                </div>
                <div className='overflow-x-auto p-6 font-mono text-sm leading-7'>
                  {codeLines.map((line, idx) => {
                    const isHighlighted = steps[activeStep].lines.includes(idx);
                    const isDimmed = !isHighlighted;

                    return (
                      <div
                        key={idx}
                        className={`flex transition-all duration-500 ${
                          isDimmed ? 'opacity-50' : 'scale-[1.01] opacity-100'
                        }`}
                      >
                        <span className='inline-block w-8 select-none pr-4 text-right text-zinc-700'>
                          {idx + 1}
                        </span>
                        <span className={`${isHighlighted ? 'font-medium text-white' : 'text-zinc-400'}`}>
                          {(() => {
                            const leadingWhitespaceMatch = line.match(/^(\s*)/);
                            const leadingWhitespace = leadingWhitespaceMatch ? leadingWhitespaceMatch[1] : '';
                            const codeContent = line.substring(leadingWhitespace.length);
                            const indentationWidth = `${leadingWhitespace.length * 1}em`;

                            return (
                              <>
                                {leadingWhitespace && (
                                  <span style={{ display: 'inline-block', width: indentationWidth }} />
                                )}
                                {codeContent
                                  .split(/(\s+|[{}[\](),.;:])/)
                                  .filter(Boolean)
                                  .map((part, i) => {
                                    const trimmed = part.trim();

                                    // Keywords
                                    if (['import', 'export', 'const', 'return'].includes(trimmed))
                                      return (
                                        <span key={i} className='text-purple-400'>
                                          {part}
                                        </span>
                                      );

                                    // Special Rivet/JS functions
                                    if (['actor'].includes(trimmed))
                                      return (
                                        <span key={i} className='text-blue-400'>
                                          {part}
                                        </span>
                                      );

                                    // Property/method names (broadcast, state, messages, push, now)
                                    if (
                                      /^[a-zA-Z_]\w*$/.test(trimmed) &&
                                      ['broadcast', 'state', 'messages', 'push', 'now', 'Date'].includes(
                                        trimmed
                                      )
                                    )
                                      return (
                                        <span key={i} className='text-blue-300'>
                                          {part}
                                        </span>
                                      );

                                    // Strings
                                    if (part.includes('"'))
                                      return (
                                        <span key={i} className='text-[#FF4500]'>
                                          {part}
                                        </span>
                                      );

                                    // Comments
                                    if (part.includes('//'))
                                      return (
                                        <span key={i} className='text-zinc-500'>
                                          {part}
                                        </span>
                                      );

                                    return part;
                                  })}
                              </>
                            );
                          })()}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          </div>

          {/* Scrolling Steps */}
          <div className='space-y-32 py-12'>
            {steps.map((step, idx) => (
              <div
                key={idx}
                data-index={idx}
                ref={el => (observerRefs.current[idx] = el)}
                className={`p-6 transition-all duration-500 ${
                  idx === activeStep ? 'opacity-100' : 'opacity-70 hover:opacity-100'
                }`}
              >
                <div className='mb-4 flex items-center gap-3'>
                  <div
                    className={`flex h-8 w-8 items-center justify-center rounded-lg text-sm font-bold transition-colors ${
                      idx === activeStep
                        ? 'border border-white/20 bg-white/10 text-white'
                        : 'bg-zinc-800 text-zinc-500'
                    }`}
                  >
                    {idx + 1}
                  </div>
                  <h3
                    className={`text-xl font-medium transition-colors ${
                      idx === activeStep ? 'text-white' : 'text-zinc-500'
                    }`}
                  >
                    {step.title}
                  </h3>
                </div>
                <p className='text-lg leading-relaxed text-zinc-400'>{step.description}</p>

                {/* Mobile Only Code Snippet */}
                <div className='mt-6 overflow-x-auto rounded-lg border border-white/10 bg-[#0A0A0A] p-4 font-mono text-xs text-zinc-300 lg:hidden'>
                  {step.lines.map(lineIdx => (
                    <div key={lineIdx} className='whitespace-pre'>
                      {codeLines[lineIdx]}
                    </div>
                  ))}
                </div>
              </div>
            ))}
            <div className='h-[20vh]' />
          </div>
        </div>
      </div>
    </section>
  );
};

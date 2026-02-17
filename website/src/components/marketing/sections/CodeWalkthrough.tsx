'use client';

import { useEffect, useState, useRef } from 'react';
import { motion } from 'framer-motion';
import { codeToHtml } from 'shiki';

const codeLines = [
  `actor({`,
  `  state: { messages: [], history: [] },`,
  ``,
  `  run: async (c) => {`,
  `    while (true) {`,
  `      const message = await c.queue.next("message");`,
  `      const response = await ai(message);`,
  `      c.state.history.push({ message, response });`,
  `      c.broadcast("response", response);`,
  `    }`,
  `  },`,
  `});`,
];

const useHighlightedCodeLines = (lines: string[]) => {
  const [highlightedLines, setHighlightedLines] = useState<string[]>([]);
  const cache = useRef<Record<string, string[]>>({});

  useEffect(() => {
    const code = lines.join('\n');
    if (cache.current[code]) {
      setHighlightedLines(cache.current[code]);
      return;
    }

    codeToHtml(code, {
      lang: 'typescript',
      theme: 'ayu-dark',
    }).then((html) => {
      const parser = new DOMParser();
      const doc = parser.parseFromString(html, 'text/html');
      const parsedLines = Array.from(doc.querySelectorAll('span.line')).map((line) => line.innerHTML);

      cache.current[code] = parsedLines;
      setHighlightedLines(parsedLines);
    });
  }, [lines]);

  return highlightedLines;
};

export const CodeWalkthrough = () => {
  const [activeStep, setActiveStep] = useState(0);
  const observerRefs = useRef<(HTMLDivElement | null)[]>([]);
  const highlightedCodeLines = useHighlightedCodeLines(codeLines);

  const steps = [
    {
      title: 'Define the Actor',
      description:
        'An Actor is an independent process with its own isolated state. Create one per user, per agent, or per session.',
      lines: [0, 11]
    },
    {
      title: 'Declare State',
      description:
        'State is automatically persisted and loaded into memory when the Actor wakes. No database queries. No ORM. Just an object.',
      lines: [1]
    },
    {
      title: 'Process Messages',
      description:
        'The run loop executes continuously. Wait for messages from a queue, process them with your logic. The Actor stays alive as long as it needs to.',
      lines: [3, 4, 5, 6, 7]
    },
    {
      title: 'Broadcast in Real-time',
      description:
        'Push updates to all connected clients instantly. WebSockets and SSE are built in — no Socket.io, no pub/sub layer, just one line.',
      lines: [8, 9, 10]
    }
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
    <section className='relative border-t border-white/10 bg-white/[0.03] pb-24 pt-48'>
      <div className='mx-auto max-w-7xl px-6'>
        {/* Mobile-only header */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='mb-12 lg:hidden'
        >
          <h2 className='mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl'>
            How Actors work.
          </h2>
          <p className='max-w-xl text-base leading-relaxed text-zinc-500'>
            Define state, write a run loop, broadcast events.
          </p>
        </motion.div>

        <div className='grid grid-cols-1 gap-16 lg:grid-cols-2'>
          {/* Sticky Code Block */}
          <div className='relative hidden lg:block'>
            <div className='sticky top-32'>
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5 }}
                className='mb-8'
              >
                <h2 className='mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl'>
                  How Actors work.
                </h2>
                <p className='max-w-xl text-base leading-relaxed text-zinc-500'>
                  Define state, write a run loop, broadcast events.
                </p>
              </motion.div>
              <div className='border-t border-white/10 pt-4'>
                <div className='mb-4 font-mono text-xs text-zinc-500'>actor.ts</div>
                <div className='overflow-x-auto font-mono text-sm leading-7'>
                  {codeLines.map((line, idx) => {
                    const isHighlighted = steps[activeStep].lines.includes(idx);
                    const isDimmed = !isHighlighted;

                    return (
                      <div
                        key={idx}
                        className={`flex transition-all duration-500 ${
                          isDimmed ? 'opacity-50' : 'opacity-100'
                        }`}
                      >
                        <span className='inline-block w-8 select-none pr-4 text-right text-zinc-700'>
                          {idx + 1}
                        </span>
                        {highlightedCodeLines[idx] ? (
                          <span
                            className={`whitespace-pre ${isHighlighted ? 'font-medium' : ''}`}
                            dangerouslySetInnerHTML={{ __html: highlightedCodeLines[idx] }}
                          />
                        ) : (
                          <span className={`whitespace-pre ${isHighlighted ? 'font-medium text-white' : 'text-zinc-400'}`}>
                            {line}
                          </span>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          </div>

          {/* Scrolling Steps */}
          <div className='space-y-64 py-24'>
            {steps.map((step, idx) => (
              <div
                key={idx}
                data-index={idx}
                ref={el => (observerRefs.current[idx] = el)}
                className={`border-t border-white/10 pt-6 transition-all duration-500 ${
                  idx === activeStep ? 'opacity-100' : 'opacity-50'
                }`}
              >
                <div className='mb-3'>
                  <span
                    className={`font-mono text-xs transition-colors ${
                      idx === activeStep ? 'text-zinc-500' : 'text-zinc-600'
                    }`}
                  >
                    {String(idx + 1).padStart(2, '0')}/{String(steps.length).padStart(2, '0')}
                  </span>
                  <h3
                    className={`mt-1 text-lg font-normal transition-colors ${
                      idx === activeStep ? 'text-white' : 'text-zinc-500'
                    }`}
                  >
                    {step.title}
                  </h3>
                </div>
                <p className='text-base leading-relaxed text-zinc-500'>{step.description}</p>

                {/* Mobile Only Code Snippet */}
                <div className='mt-6 overflow-x-auto rounded-lg border border-white/10 bg-black p-4 font-mono text-xs leading-6 lg:hidden'>
                  {step.lines.map(lineIdx => (
                    <div key={lineIdx} className='whitespace-pre text-zinc-300'>
                      {highlightedCodeLines[lineIdx] ? (
                        <span dangerouslySetInnerHTML={{ __html: highlightedCodeLines[lineIdx] }} />
                      ) : (
                        codeLines[lineIdx]
                      )}
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
};

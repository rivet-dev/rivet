'use client';

import { Zap, Database, Cpu, AlertCircle } from 'lucide-react';
import { motion } from 'framer-motion';

const FeatureCard = ({ title, description, graphic, index }) => (
  <div className='group relative flex h-full flex-col overflow-hidden rounded-3xl border border-white/10 bg-white/[0.02] backdrop-blur-sm transition-all duration-500 hover:border-white/20 hover:bg-white/[0.04] hover:shadow-[0_0_50px_-12px_rgba(255,255,255,0.1)]'>
    {/* Stylized Number */}
    <div className='absolute left-4 top-4 z-20 font-mono text-2xl font-light leading-none tracking-tight text-white/20 transition-colors group-hover:text-white/40'>
      {String(index + 1).padStart(2, '0')}
    </div>
    {/* Top Shine Highlight */}
    <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/40 to-transparent opacity-70 transition-opacity group-hover:opacity-100' />
    {/* Bottom Fade Highlight */}
    <div className='absolute bottom-0 left-0 right-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/10 to-transparent opacity-30' />
    {/* Graphic Area */}
    <div className='relative flex h-48 items-center justify-center overflow-hidden border-b border-white/5 bg-white/[0.01] transition-colors group-hover:bg-white/[0.02]'>
      {graphic}
    </div>

    {/* Content */}
    <div className='flex flex-grow flex-col p-8'>
      <h3 className='mb-3 text-xl font-medium tracking-tight text-white'>{title}</h3>
      <p className='flex-grow text-sm leading-relaxed text-zinc-400'>{description}</p>
    </div>
  </div>
);

export const FeaturesSection = () => {
  const features = [
    {
      title: 'Compute Without Timeouts',
      description:
        'Like Lambda, but memory and no timeouts. Your code can run for as long as it needs for use cases like realtime, batch jobs, and so much more.',
      graphic: (
        <div className='flex h-full w-full flex-col justify-center gap-6 px-12'>
          {/* Stateless Side */}
          <div className='flex items-center gap-4 opacity-50 transition-opacity group-hover:opacity-80'>
            <div className='w-20 text-right font-mono text-[10px] tracking-wide text-zinc-500'>stateless</div>
            <div className='relative h-[2px] flex-1 overflow-hidden rounded-full bg-zinc-800'>
              <div className='absolute inset-0 w-full origin-left animate-[statelessChurn_3s_ease-in-out_infinite] bg-zinc-400' />
            </div>
          </div>

          {/* Stateful Side */}
          <div className='flex items-center gap-4'>
            <div className='w-20 text-right font-mono text-[10px] font-bold tracking-wide text-[#FF4500]'>
              stateful
            </div>
            <div className='relative h-[2px] flex-1 overflow-hidden rounded-full bg-zinc-800'>
              <div className='absolute inset-0 w-full origin-left animate-[statefulLifecycle_6s_ease-out_infinite] bg-[#FF4500]' />
              <div className='absolute inset-0 w-full -translate-x-full animate-[shimmer_2s_infinite] bg-white/30' />
            </div>
          </div>
        </div>
      )
    },
    {
      title: 'In-Memory Speed',
      description:
        'State lives in the same place as your compute. Reads and writes are in-memory. No cache invalidation, no round-trips.',
      graphic: (
        <div className='relative flex h-full w-full items-center justify-center'>
          <div className='flex items-center gap-0'>
            {/* The Actor Box */}
            <div className='relative z-10 flex h-24 w-32 flex-col justify-between rounded-xl border border-[#FF4500]/30 bg-[#FF4500]/10 p-2 backdrop-blur-sm'>
              <div className='mb-1 text-center font-mono text-[10px] uppercase tracking-wider text-[#FF4500]'>
                Actor
              </div>
              <div className='relative flex flex-1 items-center justify-around px-1'>
                {/* Internal Pipe */}
                <div className='absolute left-8 right-8 top-1/2 -mt-2 h-1 -translate-y-1/2 overflow-hidden rounded-full bg-[#FF4500]/50'>
                  <div className='absolute inset-0 h-full w-full animate-[shuttle_1.5s_ease-in-out_infinite] bg-gradient-to-r from-transparent via-[#FF4500] to-transparent opacity-90' />
                </div>

                {/* CPU Node */}
                <div className='relative z-10 flex flex-col items-center gap-1'>
                  <div className='flex h-8 w-8 items-center justify-center rounded border border-[#FF4500]/50 bg-zinc-900 shadow-[0_0_15px_rgba(255,69,0,0.1)]'>
                    <Cpu className='h-4 w-4 text-[#FF4500]' />
                  </div>
                  <span className='text-[8px] text-[#FF4500]/70'>Compute</span>
                </div>

                {/* Local State Node */}
                <div className='relative z-10 flex flex-col items-center gap-1'>
                  <div className='flex h-8 w-8 items-center justify-center rounded border border-[#FF4500]/50 bg-zinc-900 shadow-[0_0_15px_rgba(255,69,0,0.1)]'>
                    <Database className='h-4 w-4 text-[#FF4500]' />
                  </div>
                  <span className='text-[8px] text-[#FF4500]/70'>State</span>
                </div>
              </div>
            </div>

            {/* External Pipe */}
            <div className='relative h-1 w-24 overflow-hidden rounded-full bg-zinc-800'>
              <div className='absolute inset-0 h-full w-1/2 animate-[shuttle_4s_ease-in-out_infinite] bg-gradient-to-r from-transparent via-zinc-500 to-transparent' />
            </div>

            {/* External DB */}
            <div className='flex flex-col items-center gap-1'>
              <div className='z-0 flex h-10 w-10 items-center justify-center rounded-full border border-zinc-700 bg-zinc-900 shadow-lg'>
                <Database className='h-4 w-4 text-zinc-500' />
              </div>
              <span className='text-[8px] text-zinc-600'>DB</span>
            </div>
          </div>
        </div>
      )
    },
    {
      title: 'Realtime, Built-in',
      description:
        'WebSockets out of the box. Broadcast updates with one line—no extra infrastructure, no pub/sub layer.',
      graphic: (
        <div className='relative flex h-full w-full items-center justify-center overflow-hidden'>
          {/* Center Node */}
          <div className='relative z-10 h-4 w-4 rounded-full bg-white shadow-[0_0_20px_white]' />

          {/* Ripples */}
          <div className='absolute h-16 w-16 animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite] rounded-full border border-[#FF4500]/30' />
          <div className='absolute h-32 w-32 animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite_0.5s] rounded-full border border-[#FF4500]/20' />
          <div className='absolute h-48 w-48 animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite_1s] rounded-full border border-[#FF4500]/10' />

          {/* Satellite Nodes */}
          <div className='absolute left-1/4 top-10 h-2 w-2 rounded-full bg-[#FF4500]' />
          <div className='absolute bottom-10 right-1/4 h-2 w-2 rounded-full bg-[#FF4500]' />
          <div className='absolute right-10 top-1/2 h-2 w-2 rounded-full bg-[#FF4500]' />
        </div>
      )
    },
    {
      title: 'Sleeps When Idle',
      description:
        'Actors automatically sleep to save costs and wake instantly on demand. WebSockets stay connected even while sleeping.',
      graphic: (
        <div className='relative flex h-full w-full flex-col items-center justify-center'>
          {/* Packet moves in from left */}
          <div className='absolute left-[20%] top-1/2 h-3 w-3 -translate-y-1/2 animate-[simplePacket_4s_ease-in-out_infinite] rounded-full bg-white shadow-[0_0_10px_white]' />

          {/* Actor Container */}
          <div className='relative z-10 flex flex-col items-center gap-3'>
            <div className='relative flex h-20 w-20 animate-[boxState_4s_ease-in-out_infinite] items-center justify-center overflow-hidden rounded-2xl border bg-zinc-900 shadow-2xl'>
              {/* Awake State Content (Zap) */}
              <div className='absolute inset-0 grid animate-[fadeZap_4s_ease-in-out_infinite] place-items-center'>
                <Zap className='h-10 w-10 fill-yellow-400/20 text-yellow-400 drop-shadow-[0_0_15px_rgba(250,204,21,0.6)]' />
              </div>

              {/* Sleep State Content (Zzz) */}
              <div className='absolute inset-0 grid animate-[fadeZzz_4s_ease-in-out_infinite] place-items-center'>
                <div className='mb-1 flex items-end gap-[1px]'>
                  <span
                    className='animate-[float_3s_ease-in-out_infinite] text-2xl font-bold text-zinc-600'
                    style={{ animationDelay: '0s' }}
                  >
                    Z
                  </span>
                  <span
                    className='animate-[float_3s_ease-in-out_infinite] text-xl font-bold text-zinc-700'
                    style={{ animationDelay: '0.5s' }}
                  >
                    z
                  </span>
                  <span
                    className='animate-[float_3s_ease-in-out_infinite] text-sm font-bold text-zinc-800'
                    style={{ animationDelay: '1s' }}
                  >
                    z
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      )
    },
    {
      title: 'Resilient by Design',
      description:
        'Automatic failover maintain 100% fault tolerance. Your actors survive crashes and upgrades.',
      graphic: (
        <div className='relative flex h-full w-full items-center justify-center'>
          {/* Pulse Ring Background */}
          <div className='absolute h-32 w-32 animate-[ping_3s_cubic-bezier(0,0,0.2,1)_infinite] rounded-full bg-[#FF4500]/5' />

          {/* The Process Shell */}
          <div className='relative z-10 flex h-16 w-16 animate-[shellRecover_4s_ease-in-out_infinite] items-center justify-center rounded-xl border-2 border-solid bg-zinc-900'>
            {/* The Persistent State (Core) */}
            <div className='z-20 text-white'>
              <Database className='h-6 w-6 fill-[#FF4500]/20 text-[#FF4500] drop-shadow-[0_0_10px_rgba(255,69,0,0.5)]' />
            </div>

            {/* Crash indicator overlay (The 'X' or Alert) */}
            <div className='absolute -right-3 -top-3 z-30 flex animate-[crashIcon_4s_ease-in-out_infinite] items-center justify-center rounded-full border border-red-500/50 bg-zinc-900 p-1 text-red-500 opacity-0 shadow-lg shadow-red-500/20'>
              <AlertCircle className='h-5 w-5 fill-red-500/20' />
            </div>
          </div>

          {/* "Rebooting" Spinner ring appearing during crash */}
          <div className='absolute inset-0 m-auto h-24 w-24 animate-[spinRecover_4s_ease-in-out_infinite] rounded-full border-2 border-[#FF4500]/50 border-t-[#FF4500] opacity-0' />
        </div>
      )
    },
    {
      title: 'Multi-Region',
      description:
        'Deploy actors across regions worldwide. Compute and state live together at the edge, delivering ultra-low latency responses — something stateless edge runtimes can\'t match.',
      graphic: (
        <div className='relative flex h-full w-full items-center justify-center perspective-[600px]'>
          {/* Tilted Plane */}
          <div className='relative h-32 w-48 transform rounded-xl border border-white/10 bg-white/5 shadow-2xl [transform:rotateX(60deg)]'>
            {/* Grid lines on plane */}
            <div className='absolute inset-0 bg-[linear-gradient(to_right,rgba(255,255,255,0.05)_1px,transparent_1px),linear-gradient(to_bottom,rgba(255,255,255,0.05)_1px,transparent_1px)] bg-[size:16px_16px]' />

            {/* Region Nodes (positioned on the plane) */}

            {/* us-west-1 (Oregon) */}
            <div className='absolute left-[20%] top-[40%]'>
              <div className='h-2 w-2 rounded-full bg-[#FF4500] shadow-[0_0_10px_#FF4500]' />
              {/* Beam */}
              <div className='absolute bottom-full left-1/2 h-16 w-[1px] -translate-x-1/2 animate-[beam_2s_ease-in-out_infinite] bg-gradient-to-t from-[#FF4500] to-transparent' />
              <div className='absolute bottom-full left-1/2 -translate-x-1/2 -translate-y-16 animate-[fadeInUp_2s_ease-in-out_infinite] font-mono text-[8px] text-orange-200 opacity-0'>
                OR
              </div>
            </div>

            {/* us-east-1 (N. Virginia) */}
            <div className='absolute left-[45%] top-[30%]'>
              <div className='h-2 w-2 rounded-full bg-[#FF4500] shadow-[0_0_10px_#FF4500] delay-300' />
              <div className='absolute bottom-full left-1/2 h-16 w-[1px] -translate-x-1/2 animate-[beam_2s_ease-in-out_infinite_0.5s] bg-gradient-to-t from-[#FF4500] to-transparent' />
              <div className='absolute bottom-full left-1/2 -translate-x-1/2 -translate-y-16 animate-[fadeInUp_2s_ease-in-out_infinite_0.5s] font-mono text-[8px] text-orange-200 opacity-0'>
                VA
              </div>
            </div>

            {/* eu-central-1 (Frankfurt) */}
            <div className='absolute left-[65%] top-[25%]'>
              <div className='h-2 w-2 rounded-full bg-[#FF4500] shadow-[0_0_10px_#FF4500] delay-500' />
              <div className='absolute bottom-full left-1/2 h-16 w-[1px] -translate-x-1/2 animate-[beam_2s_ease-in-out_infinite_1s] bg-gradient-to-t from-[#FF4500] to-transparent' />
              <div className='absolute bottom-full left-1/2 -translate-x-1/2 -translate-y-16 animate-[fadeInUp_2s_ease-in-out_infinite_1s] font-mono text-[8px] text-orange-200 opacity-0'>
                FRA
              </div>
            </div>

            {/* ap-southeast-1 (Singapore) */}
            <div className='absolute left-[85%] top-[45%]'>
              <div className='h-2 w-2 rounded-full bg-[#FF4500] shadow-[0_0_10px_#FF4500] delay-700' />
              <div className='absolute bottom-full left-1/2 h-16 w-[1px] -translate-x-1/2 animate-[beam_2s_ease-in-out_infinite_1.5s] bg-gradient-to-t from-[#FF4500] to-transparent' />
              <div className='absolute bottom-full left-1/2 -translate-x-1/2 -translate-y-16 animate-[fadeInUp_2s_ease-in-out_infinite_1.5s] font-mono text-[8px] text-orange-200 opacity-0'>
                SG
              </div>
            </div>
          </div>
        </div>
      )
    },
    // {
    //   title: 'Open Source & Self-Hostable',
    //   description:
    //     'No lock-in. Run on your platform of choice or bare metal with the same API and mental model.',
    //   graphic: (
    //     <div className='flex h-32 w-48 rotate-3 transform flex-col gap-1.5 rounded-lg border border-white/10 bg-zinc-950 p-3 font-mono text-[10px] text-zinc-500 shadow-2xl transition-transform duration-500 hover:rotate-0'>
    //       <div className='mb-1 flex gap-1.5 opacity-50'>
    //         <div className='h-2 w-2 rounded-full bg-white' />
    //         <div className='h-2 w-2 rounded-full bg-white' />
    //         <div className='h-2 w-2 rounded-full bg-white' />
    //       </div>
    //       <div className='flex gap-2 text-[#FF4500]'>
    //         <span className='select-none'>&gt;</span> rivet-engine start
    //       </div>
    //       <div>Starting rivet...</div>
    //       <div className='text-zinc-300'>Listening on port 6420</div>
    //       <div className='mt-1 flex items-center gap-1 text-[#FF4500]'>
    //         <span>&gt;</span>
    //         <span className='h-3 w-1.5 animate-pulse bg-[#FF4500]' />
    //       </div>
    //     </div>
    //   )
    // }
  ];

  return (
    <section id='features' className='relative bg-black py-32'>
      <div className='mx-auto max-w-7xl px-6'>
        <div className='mb-20 max-w-2xl'>
          <motion.h2
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className='mb-6 text-3xl font-medium tracking-tight text-white md:text-5xl'
          >
            Everything you need for
            <br />
            stateful workloads.
          </motion.h2>
          <motion.p
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className='text-lg leading-relaxed text-zinc-400'
          >
            Rivet handles the hard parts of distributed systems: scaling, fault tolerance, and realtime. You
            just write the logic.
          </motion.p>
        </div>

        <div className='grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3'>
          {features.map((feature, idx) => (
            <motion.div
              key={idx}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: idx * 0.1 }}
            >
              <FeatureCard {...feature} index={idx} />
            </motion.div>
          ))}
        </div>
      </div>

      {/* Custom Animations for Graphics */}
      <style>{`
        @keyframes statelessChurn {
          0% { transform: scaleX(0); background-color: rgb(161 161 170); }
          20% { transform: scaleX(1); background-color: rgb(161 161 170); }
          50% { transform: scaleX(1); background-color: rgb(161 161 170); opacity: 1; }
          55% { transform: scaleX(1); background-color: rgb(239 68 68); opacity: 1; }
          60% { transform: scaleX(1); opacity: 0; }
          100% { transform: scaleX(1); opacity: 0; }
        }

        @keyframes statefulLifecycle {
          0% { transform: scaleX(0); opacity: 1; }
          10% { transform: scaleX(1); opacity: 1; }
          30% { opacity: 1; }
          40% { opacity: 0.3; }
          80% { opacity: 0.3; }
          90% { opacity: 1; }
          100% { opacity: 1; }
        }

        @keyframes shimmer {
          0% { transform: translateX(-100%); }
          100% { transform: translateX(100%); }
        }

        @keyframes shuttle {
          0% { transform: translateX(-100%); }
          50% { transform: translateX(100%); }
          100% { transform: translateX(-100%); }
        }

        @keyframes simplePacket {
          0% { left: 15%; opacity: 0; transform: translate(-50%, -50%) scale(0.5); }
          10% { opacity: 1; transform: translate(-50%, -50%) scale(1); }
          40% { left: 50%; opacity: 1; transform: translate(-50%, -50%) scale(1); }
          42% { left: 50%; opacity: 0; transform: translate(-50%, -50%) scale(1.2); }
          100% { left: 50%; opacity: 0; }
        }

        @keyframes boxState {
          0%, 39% { border-color: rgb(39 39 42); box-shadow: none; }
          40%, 90% { border-color: rgb(255, 69, 0); box-shadow: 0 0 20px -5px rgba(255, 69, 0, 0.3); }
          95%, 100% { border-color: rgb(39 39 42); box-shadow: none; }
        }

        @keyframes fadeZap {
          0%, 39% { opacity: 0; transform: scale(0.9); }
          42%, 90% { opacity: 1; transform: scale(1); }
          95%, 100% { opacity: 0; transform: scale(0.9); }
        }

        @keyframes fadeZzz {
          0%, 39% { opacity: 1; transform: scale(1); }
          42%, 90% { opacity: 0; transform: scale(0.9); }
          95%, 100% { opacity: 1; transform: scale(1); }
        }

        @keyframes float {
          0%, 100% { transform: translateY(0); }
          50% { transform: translateY(-4px); }
        }

        @keyframes shellRecover {
          0%, 47% { border-color: rgb(255, 69, 0); border-style: solid; }
          48%, 59% { border-color: rgb(239 68 68); border-style: dashed; }
          60% { border-color: transparent; border-style: solid; }
          70%, 100% { border-color: rgb(255, 69, 0); border-style: solid; }
        }

        @keyframes crashIcon {
          0%, 48% { opacity: 0; transform: scale(0.5); }
          50%, 55% { opacity: 1; transform: scale(1.2); }
          60%, 100% { opacity: 0; transform: scale(0.5); }
        }

        @keyframes spinRecover {
          0%, 55% { opacity: 0; transform: rotate(0deg); }
          60% { opacity: 1; }
          70% { opacity: 1; transform: rotate(360deg); }
          80%, 100% { opacity: 0; transform: rotate(360deg); }
        }

        @keyframes beam {
          0%, 100% { height: 0; opacity: 0; }
          50% { height: 64px; opacity: 1; }
        }

        @keyframes fadeInUp {
          0%, 100% { opacity: 0; transform: translate(-50%, 0); }
          50% { opacity: 1; transform: translate(-50%, -48px); }
        }
      `}</style>
    </section>
  );
};

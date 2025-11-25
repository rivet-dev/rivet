import { Icon, faArrowRight, faShapes, faSquareQuestion } from '@rivet-gg/icons';
import Link from 'next/link';

export function LibrariesGrid() {
  return (
    <div className='mx-auto grid max-w-4xl grid-cols-1 gap-4 md:grid-cols-2'>
      <Link href='/docs/actors' className='group block'>
        <div className='bg-white/2 relative flex h-[200px] flex-col overflow-hidden rounded-xl border border-white/20 shadow-sm transition-all duration-200 group-hover:border-white/40'>
          <div className='mt-6 px-6'>
            <div className='mb-4 flex items-center justify-between'>
              <div className='flex items-center gap-3 text-base text-white'>
                <Icon icon={faShapes} />
                <h3 className='font-medium'>Actors</h3>
              </div>
              <div className='opacity-0 transition-opacity group-hover:opacity-100'>
                <Icon
                  icon={faArrowRight}
                  className='-translate-x-1 text-xl text-white/80 transition-all group-hover:translate-x-0'
                />
              </div>
            </div>
            <div className='space-y-3'>
              <p className='text-base leading-relaxed text-white/40'>
                Long running tasks with state persistence, hibernation, and realtime
              </p>
              <p className='text-sm text-white/30'>
                Replaces <span className='font-medium text-white/60'>Durable Objects</span>,{' '}
                <span className='font-medium text-white/60'>Orleans</span>, or{' '}
                <span className='font-medium text-white/60'>Akka</span>
              </p>
            </div>
          </div>
        </div>
      </Link>

      <div className='bg-white/2 relative flex h-[200px] flex-col overflow-hidden rounded-xl border border-white/20 shadow-sm'>
        <div className='mt-6 px-6'>
          <div className='mb-4 flex items-center gap-3 text-base text-white'>
            <Icon icon={faSquareQuestion} />
            <h3 className='font-medium'>Coming Soon</h3>
          </div>
          <div className='space-y-3'>
            <p className='text-base leading-relaxed text-white/40'>Stay tuned for more</p>
          </div>
        </div>
      </div>
    </div>
  );
}

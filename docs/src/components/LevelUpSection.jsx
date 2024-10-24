import Image from 'next/image';
import { Button } from '@/components/Button';
import imgBlockLeft from '@/images/effects/leftBlock.png';
import imgBlockRight from '@/images/effects/rightBlock.png';
import { Icon, faPhone, faRocketLaunch } from '@rivet-gg/icons';

export default function LevelUpSection() {
  return (
    <div className='w-full bg-white p-8 text-black md:h-[600px]'>
      <div className='mx-auto flex h-full max-w-screen-xl flex-row items-center justify-between'>
        {/* Left Image Container */}
        <div className='flex h-full flex-1 items-center justify-end'>
          <Image
            src={imgBlockLeft}
            alt='Left Image'
            className='h-auto w-full' // Adjust size as necessary
          />
        </div>

        {/* Text Container - Centered Vertically and Horizontally */}
        <div className='mx-8 flex h-full flex-col items-center justify-center text-center'>
          <div className='mb-5 font-display text-6xl font-bold md:text-8xl'>Level Up with Rivet</div>
          <div className='mb-5 mt-2 font-display text-2xl font-bold italic md:text-4xl'>
            and get back to game development
          </div>

          <div className='mt-10 flex flex-wrap items-center justify-center gap-x-6 gap-y-8'>
            <Button variant='blackJuicy' href='https://hub.rivet.gg'>
              <Icon icon={faRocketLaunch} className='mr-2' />
              Get Started
            </Button>
            <Button
              variant='blackJuicy'
              href='https://calendly.com/nicholas_kissel/rivet-demo'
              target='_blank'>
              <Icon icon={faPhone} className='mr-2' />
              Request a Demo
            </Button>
          </div>
        </div>

        {/* Right Image Container */}
        <div className='flex h-full flex-1 items-center justify-start'>
          <Image
            src={imgBlockRight}
            alt='Right Image'
            className='h-auto w-full' // Adjust size as necessary
          />
        </div>
      </div>
    </div>
  );
}

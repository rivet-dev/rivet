import { MarketingButton } from '../components/MarketingButton';
import { AnimatedCTATitle } from '../components/AnimatedCTATitle';

export function CTASection() {
  return (
    <div className='mx-auto max-w-4xl text-center'>
      <AnimatedCTATitle />

      <div className='h-8' />

      <div className='mb-4 flex flex-col justify-center gap-4 sm:flex-row'>
        <MarketingButton href='/docs/quickstart/' primary>
          Get Started
        </MarketingButton>
        <MarketingButton href='/talk-to-an-engineer'>Talk to an engineer</MarketingButton>
      </div>
    </div>
  );
}

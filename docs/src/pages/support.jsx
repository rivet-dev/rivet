import { faCalendar, faEnvelope, faDiscord, faHeartbeat } from '@rivet-gg/icons';
import { Resource } from '@/components/Resources';

export default function Support() {
  return (
    <div className='pb-8'>
      <h1>Support</h1>
      <p>We love talking to developers and gamers alike. Reach out!</p>
      <div className='not-prose mt-4 grid grid-cols-1 gap-8 border-t border-charcole-900/5 pt-10 dark:border-white/5 sm:grid-cols-2 xl:grid-cols-4'>
        <Resource title='Discord' icon={faDiscord} href='https://discord.gg/aXYfyNxYVn' target='_blank'>
          Best for technical support & just saying hi
        </Resource>
        <Resource
          title='Book a Demo'
          icon={faCalendar}
          href='https://calendly.com/d/zvq-v4z-84t/rivet-founders-15-minute'
          target='_blank'>
          Best for sales & partnership inquiries
        </Resource>
        <Resource title='Email' icon={faEnvelope} href='mailto:support@rivet.gg'>
          Best for everything else
        </Resource>
        <Resource
          title='Status Page'
          icon={faHeartbeat}
          href='https://rivet-gg.betteruptime.com/'
          target='_blank'></Resource>
      </div>
    </div>
  );
}

Support.fullWidth = true;

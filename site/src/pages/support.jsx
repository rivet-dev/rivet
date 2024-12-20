import { faCalendar, faEnvelope, faDiscord, faHeartbeat } from '@rivet-gg/icons';
import { Resource } from '@/components/Resources';

export default function Support() {
  return (
    <div className='pb-8'>
      <h1>Support</h1>
      <div className='not-prose mt-4 grid grid-cols-1 gap-8 border-t border-charcole-900/5 pt-10 dark:border-white/5 sm:grid-cols-2 xl:grid-cols-4'>
        <Resource title='Discord' icon={faDiscord} href='https://discord.gg/aXYfyNxYVn' target='_blank'>
          Best for technical support & just saying hi
        </Resource>
        <Resource
          title='Book a Demo'
          icon={faCalendar}
          href='/sales'>
          Best for sales & partnership inquiries
        </Resource>
        <Resource title='Email' icon={faEnvelope} href='mailto:support@rivet.gg'>
          Best for everything else
        </Resource>
        <Resource
          title='Status Page'
          icon={faHeartbeat}
          href='https://rivet.betteruptime.com/'
          target='_blank' />
      </div>
    </div>
  );
}

Support.fullWidth = true;

import { Icon, faBluesky, faDiscord, faGithub, faXTwitter } from '@rivet-gg/icons';

export function CommunitySection() {
  const communityLinks = [
    {
      href: 'https://rivet.dev/discord',
      icon: faDiscord,
      label: 'Discord'
    },
    {
      href: 'https://x.com/rivet_dev',
      icon: faXTwitter,
      label: 'X'
    },
    {
      href: 'https://bsky.app/profile/rivet.gg',
      icon: faBluesky,
      label: 'Bluesky'
    },
    {
      href: 'https://github.com/rivet-dev/rivetkit/discussions',
      icon: faGithub,
      label: 'Discussions'
    },
    {
      href: 'https://github.com/rivet-dev/rivetkit/issues',
      icon: faGithub,
      label: 'Issues'
    }
  ];

  return (
    <div className='mx-auto max-w-6xl text-center'>
      <div className='mb-16'>
        <h2 className='font-700 mb-6 text-2xl text-white sm:text-3xl'>Join the Community</h2>
        <p className='font-500 mx-auto max-w-2xl text-lg text-white/40'>
          Join thousands of developers building with Rivet Actors today
        </p>
      </div>

      <div className='mx-auto grid max-w-4xl grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-5'>
        {communityLinks.map((link, index) => (
          <a
            key={index}
            href={link.href}
            className='bg-white/2 group flex flex-col items-center gap-3 rounded-xl border border-white/20 px-4 py-6 transition-all duration-200 hover:border-white/40 hover:bg-white/5'
            target='_blank'
            rel='noopener noreferrer'
          >
            <Icon icon={link.icon} className='h-6 w-6 text-white' />
            <span className='text-sm font-medium text-white'>{link.label}</span>
          </a>
        ))}
      </div>
    </div>
  );
}

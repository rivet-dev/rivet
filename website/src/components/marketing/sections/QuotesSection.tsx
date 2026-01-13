import { Icon, faArrowRight } from '@rivet-gg/icons';

import chinomanAvatar from '../images/quotes/users/Chinoman10_.jpg';
import socialQuotientAvatar from '../images/quotes/users/Social_Quotient.jpg';
import alistaiirAvatar from '../images/quotes/users/alistaiir.jpg';
// User avatars
import devgerredAvatar from '../images/quotes/users/devgerred.jpg';
import j0g1tAvatar from '../images/quotes/users/j0g1t.jpg';
import localFirstAvatar from '../images/quotes/users/localfirstnews.jpg';
import samgoodwinAvatar from '../images/quotes/users/samgoodwin89.jpg';
import samk0Avatar from '../images/quotes/users/samk0_com.jpg';
import uripontAvatar from '../images/quotes/users/uripont_.jpg';

import j0g1tPostImage from '../images/quotes/posts/1902835527977439591.jpg';
// Post images
import samk0PostImage from '../images/quotes/posts/1909278348812952007.png';

export function QuotesSection() {
  const quotesColumn1 = [
    {
      href: 'https://x.com/devgerred/status/1903178025598083285',
      avatar: devgerredAvatar,
      name: 'gerred',
      handle: '@devgerred',
      content: 'Nice work, @rivet_dev - nailed it'
    },
    {
      href: 'https://x.com/samk0_com/status/1909278348812952007',
      avatar: samk0Avatar,
      name: 'Samo',
      handle: '@samk0_com',
      content: 'Great UX & DX possible thanks to @RivetKit_org',
      image: samk0PostImage
    },
    {
      href: 'https://x.com/Social_Quotient/status/1903172142121832905',
      avatar: socialQuotientAvatar,
      name: 'John Curtis',
      handle: '@Social_Quotient',
      content: 'Loving RivetKit direction!'
    },
    {
      href: 'https://x.com/localfirstnews/status/1902752173928427542',
      avatar: localFirstAvatar,
      name: 'Local-First Newsletter',
      handle: '@localfirstnews',
      content: 'Featured in newsletter',
      isItalic: true
    },
    {
      href: 'https://x.com/Chinoman10_/status/1902020312306216984',
      avatar: chinomanAvatar,
      name: 'Chinomso',
      handle: '@Chinoman10_',
      content:
        "Alternatively, some dude (@NathanFlurry) recently told me about @RivetKit_org, which optionally brings you vendor-flexibility (no lock-in since it's abstracted for you)."
    }
  ];

  const quotesColumn2 = [
    {
      href: 'https://x.com/uripont_/status/1910817946470916525',
      avatar: uripontAvatar,
      name: 'uripont',
      handle: '@uripont_',
      content:
        'Crazy to think that there are so many things to highlight that is actually hard to convey it in a few words.'
    },
    {
      href: 'https://x.com/samgoodwin89/status/1910791029609091456',
      avatar: samgoodwinAvatar,
      name: 'sam',
      handle: '@samgoodwin89',
      content: '"Durable Objects without the boilerplate"'
    },
    {
      href: 'https://x.com/j0g1t/status/1902835527977439591',
      avatar: j0g1tAvatar,
      name: 'Kacper Wojciechowski',
      handle: '@j0g1t',
      content: 'Your outie uses @RivetKit_org to develop realtime applications.',
      image: j0g1tPostImage
    },
    {
      href: 'https://x.com/alistaiir/status/1891312940302716984',
      avatar: alistaiirAvatar,
      name: 'alistair',
      handle: '@alistaiir',
      content: 'RivetKit looks super awesome.'
    }
  ];

  const QuoteCard = ({ quote }: { quote: any }) => (
    <a href={quote.href}
      className='bg-white/2 group block rounded-xl border border-white/20 p-6 transition-all duration-200 hover:border-white/40 hover:bg-white/10'
      target='_blank'
      rel='noopener noreferrer'
    >
      <div className='mb-4 flex items-start gap-3'>
        <img src={quote.avatar}
          alt={quote.name}
          width={40}
          height={40}
          className='rounded-full object-cover'
        />
        <div className='min-w-0 flex-1'>
          <p className='text-sm font-medium text-white'>{quote.name}</p>
          <p className='text-sm text-white/40'>{quote.handle}</p>
        </div>
      </div>
      <p className={`font-500 mb-4 leading-relaxed text-white/40 ${quote.isItalic ? 'italic' : ''}`}>
        {quote.content}
      </p>
      {quote.image && (
        <img src={quote.image}
          alt='Tweet media'
          width={300}
          height={200}
          className='w-full rounded-lg object-cover'
        />
      )}
    </a>
  );

  return (
    <div className='mx-auto max-w-6xl'>
      <div className='mb-16 text-center'>
        <h2 className='font-700 mb-6 text-2xl text-white sm:text-3xl'>What People Are Saying</h2>
        <p className='font-500 text-lg text-white/40'>From the platform formerly known as Twitter</p>
      </div>

      <div className='mb-16 grid grid-cols-1 gap-6 lg:grid-cols-2'>
        {/* Column 1 */}
        <div className='space-y-6'>
          {quotesColumn1.map((quote, index) => (
            <QuoteCard key={index} quote={quote} />
          ))}
        </div>

        {/* Column 2 */}
        <div className='space-y-6'>
          {quotesColumn2.map((quote, index) => (
            <QuoteCard key={index} quote={quote} />
          ))}
        </div>
      </div>

      {/* Tweet Button */}
      <div className='text-center'>
        <a href='https://twitter.com/intent/tweet?text=%40RivetKit_org%20'
          className='bg-white/2 inline-flex items-center gap-2 rounded-lg border border-white/20 px-4 py-2 font-medium text-white transition-all duration-200 hover:border-white/40 hover:bg-white/10'
          target='_blank'
          rel='noopener noreferrer'
        >
          Share your feedback on X
          <Icon icon={faArrowRight} className='h-4 w-4' />
        </a>
      </div>
    </div>
  );
}

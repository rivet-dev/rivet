import React from 'react';
import clsx from 'clsx';
import { Icon, faServer, faChess, faAddressCard, faPuzzle, faArrowRight } from '@rivet-gg/icons';
import { Tooltip } from '@/components/mdx';

export default function MainFeatures() {
  return (
    <div
      className={clsx(
        'mx-auto w-full max-w-7xl gap-8 px-4 sm:gap-20 sm:px-12',
        'grid grid-cols-1 md:grid-cols-2'
      )}>
      <MainFeatureColumn
        icon={faServer}
        title='Dedicated Game Server Hosting'
        description={
          <>
            Autoscaling &{' '}
            <Tooltip tip='Boot servers on-demand in less than 5 seconds'>instant servers</Tooltip>. Includes
            DDoS mitigation, monitoring, & crash reporting. Supports TCP, UDP, WebSockets, &{' '}
            <Tooltip tip='WebRTC, ENet, KPC'>more</Tooltip>.
          </>
        }
        buttons={[{ name: 'Documentation', href: '/docs/dynamic-servers' }]}
      />
      <MainFeatureColumn
        icon={faChess}
        title='Matchmaking, Lobbies, & Parties'
        description='Supports casual, competitive, MMO, and turn-based. Works with server-authoritative, P2P, & async multiplayer.'
        buttons={[{ name: 'Documentation', href: '/docs/matchmaker' }]}
      />
      <MainFeatureColumn
        icon={faAddressCard}
        title='Accounts, Friends, & Presence'
        description={
          <>
            Display friends online & facilitate playing together. Authenticate with email, username, or{' '}
            <Tooltip tip='Google, Twitch, Discord, and more'>social</Tooltip>.
          </>
        }
        buttons={[{ name: 'Documentation', href: '/modules/auth/overview', target: '_blank' }]}
      />
      <MainFeatureColumn
        icon={faPuzzle}
        title='100% Modular & Scriptable'
        description={
          <>
            Pick and choose modules to use. Easily write server-side scripts & real-time actors.{' '}
            <Tooltip tip='Powered by Postgres'>Database</Tooltip> included for free.
          </>
        }
        buttons={[{ name: 'Documentation', href: '/docs/modules/quickstart', target: '_blank' }]}
      />
    </div>
  );
}

function MainFeatureColumn({ icon, title, description, buttons }) {
  return (
    <div className={clsx('relative', 'h-full text-left', 'flex flex-col', 'col-span-1')}>
      <div className='flex h-full items-start space-x-4'>
        <div className='flex-shrink-0'>
          <div className='flex h-14 w-14 items-center justify-center rounded-xl border-2 border-charcole-500/5 bg-charcole-600/10'>
            <Icon icon={icon} className='text-xl text-cream-100' />
          </div>
        </div>
        <div className='flex h-full flex-grow flex-col'>
          <h2 className='font-display text-2xl font-semibold text-cream-100'>{title}</h2>
          <div className='mt-2 max-w-2xl'>
            <p className='text-sm font-semibold text-cream-100/90'>{description}</p>
          </div>
          <div className='min-h-2 flex-grow'></div>
          <div className='flex flex-col gap-2'>
            {buttons.map((button, i) => (
              <a
                key={i}
                href={button.href}
                target={button.target}
                rel='noreferrer'
                className={clsx(
                  'justify-left flex items-center gap-1 text-xs font-bold text-orange-400 hover:text-orange-300 sm:text-sm',
                  button.classes
                )}>
                {button.name}
                <Icon icon={faArrowRight} className='h-6 w-6' />
              </a>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

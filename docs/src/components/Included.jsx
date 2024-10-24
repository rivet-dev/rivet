import React from 'react';
import { Tooltip } from '@/components/mdx';
import {
  Icon,
  faShield,
  faClockRotateLeft,
  faWarehouse,
  faBars,
  faLock,
  faMessageExclamation
} from '@rivet-gg/icons';
import clsx from 'clsx';

export function IncludedFeature({ icon, title, usually, on }) {
  return (
    <div className='flex flex-col items-start border border-cream-100/5 px-10 py-12 text-left'>
      {/* Icon */}
      <Icon icon={icon} className='mb-4 text-2xl text-cream-100/80' />

      {/* Content */}
      <div>
        <div className='text-lg font-semibold leading-tight text-cream-100'>{title}</div>
        {usually && (
          <div className='mt-2 text-sm font-semibold leading-tight text-cream-100/50'>
            Usually <Tooltip tip={on}>{usually}</Tooltip>
          </div>
        )}
      </div>
    </div>
  );
}

const OTHER_SERVICES_SERVER_COUNT = 15;
const OTHER_SERVICES_USER_COUNT = 4;
const OTHER_SERVICES_PRICE =
  (10 + 0.5) * OTHER_SERVICES_SERVER_COUNT + 9 * OTHER_SERVICES_USER_COUNT + 26 + 40;

export default function IncludedSection() {
  const receiptWidth = 29;
  const otherServicesText = 'Other services';
  const otherServicesPrice = `~\$${OTHER_SERVICES_PRICE.toFixed(2)}/mo`;
  return (
    <div className='mx-auto flex max-w-6xl flex-col px-4'>
      <h2 className='text-center font-display text-4xl font-bold tracking-tight text-cream-100 sm:text-5xl'>
        {'Included for Free'}
      </h2>
      <div className='h-20' />
      <div
        className={clsx(
          'grid w-full',
          'grid-cols-1',
          'sm:grid-cols-2',
          'lg:grid-cols-3',
          'border border-cream-100/5'
        )}>
        <IncludedFeature
          icon={faShield}
          title='DDoS Mitigation'
          usually='$10/server/mo'
          on='Vultr DDoS mitigation'
        />
        <IncludedFeature
          icon={faClockRotateLeft}
          title='No downtime deploys & instant rollbacks'
          usually='$40/mo'
          on='DigitalOcean Kubernetes HA Control Plane'
        />
        <IncludedFeature
          icon={faWarehouse}
          title='Version history for rollbacks'
          usually='$9/user/mo'
          on='Docker Hub'
        />
        <IncludedFeature
          icon={faBars}
          title='Log & metrics aggregation'
          usually='$0.50/server/mo'
          on='Grafana Cloud Loki with 1 log/server/second'
        />
        <IncludedFeature icon={faLock} title='Automatic SSL for WebSockets & TLS' />
        <IncludedFeature
          icon={faMessageExclamation}
          title='Crash reporting'
          usually='$26/mo'
          on='Sentry Team'
        />
        <div className='text-md col-span-full flex flex-col items-center whitespace-pre border border-cream-100/5 px-10 py-12 font-mono text-cream-100 md:text-xl'>
          <div>
            <span className='font-bold'>{otherServicesText}</span>
            {' '.repeat(receiptWidth - otherServicesText.length - otherServicesPrice.length)}
            <Tooltip tip='Assuming 15 servers & 4 users'>{otherServicesPrice}</Tooltip>
          </div>
          <div>
            <span className='font-bold'>Rivet</span> $0.00
          </div>
        </div>
      </div>
    </div>
  );
}

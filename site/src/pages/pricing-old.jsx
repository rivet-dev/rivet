import fs from 'fs';
import path from 'path';

import { Tooltip } from '@/components/mdx';
import clsx from 'clsx';
import { Button } from '@/components/Button';
import LevelUpSection from '@/components/LevelUpSection';
import { useEffect, useState } from 'react';
import grainDark from '@/images/effects/grain-dark.png';
import TimeSeriesChart from '@/components/TimeSeriesChart';

import { Icon, faCoin, faArrowRight } from '@rivet-gg/icons';
import IncludedSection from '../components/Included';

export async function getStaticProps() {
  const filePath = path.join(process.cwd(), 'public', 'pricing', 'autoscaling-data.csv');
  const csvData = fs.readFileSync(filePath, 'utf8');

  // Parse data
  let lines = csvData.split('\n').map(line => line.split(','));
  const headersRaw = lines.shift();
  const headers = headersRaw.slice(1); // Get labels from the first row, excluding 'Time'
  let labels = lines.map(row => parseInt(row[0]));

  // Select slice
  const minDate = 1712610900 * 1000;
  const maxDate = minDate + 24 * 60 * 60 * 1000 * 3;
  const minIndex = labels.findIndex(x => x > minDate);
  const maxIndex = labels.findIndex(x => x > maxDate);
  labels = labels.slice(minIndex, maxIndex);
  lines = lines.slice(minIndex, maxIndex);

  // Sum & normalize values
  let values = lines
    // Sum the row of values
    .map(row => {
      return row
        .slice(1)
        .map(x => parseInt(x) || 0)
        .reduce((a, c) => a + c, 0);
    });
  let maxValue = values.reduce((acc, curr) => Math.max(acc, curr), 0);
  values = values.map(x => x / maxValue);

  return {
    props: {
      autoscalingData: {
        labels,
        values
      }
    }
  };
}

export default function Pricing({ autoscalingData }) {
  return (
    <>
      {/* Header */}
      <h1 className='mt-8 text-center text-4xl font-bold tracking-tight text-cream-100 sm:text-5xl'>
        <Icon icon={faCoin} className='mr-2 h-10 w-10' /> Pricing
      </h1>
      <p className='mt-5 text-center font-display text-3xl tracking-tight text-cream-100/80'>
        {'Affordable for small studios & scalable for large studios.'}
        <br />
        {'Always predictable pricing.'}
      </p>

      {/* Cards */}
      <div
        className={clsx(
          'mx-auto mt-24 grid max-w-[1800px]',
          'grid-cols-1 gap-4 px-4',
          'sm:grid-cols-2',
          'md:px-6',
          'lg:grid-cols-4'
        )}>
        <PricingCard
          title='Free'
          price='Free'
          features={[
            'Great for early development & game jams',
            '$5/mo in server credits',
            'Supports US (Los Angeles), EU (Frankfurt), and Asia (Tokyo)',
            'Max 1 server'
          ]}
          options={
            <Button variant='primaryJuicy' href='https://hub.rivet.gg'>
              Get Started <Icon icon={faArrowRight} className='h-6 w-6' />
            </Button>
          }
        />

        <PricingCard
          title='Indie'
          price='$9/mo'
          features={[
            'Fixed price suitable for indies & hobbyists',
            // <><Tooltip tip={`Provided as ${formatUSD(INDIE_CREDITS)} in credits`}>3 Flex or {Math.floor(INDIE_CREDITS / STANDARD_SHARED_UNIT)} Standard servers</Tooltip> included</>,
            <>
              <Tooltip
                tip={`Specifically Flex Shared ½ servers. Provided as ${formatUSD(
                  INDIE_CREDITS
                )} in server credits. Credits can be used for any type of Flex or Standard server.`}>
                6 Flex servers
              </Tooltip>{' '}
              included
            </>,
            'Supports US (Los Angeles), EU (Frankfurt), and Asia (Tokyo)'
          ]}
          options={
            <Button variant='primaryJuicy' href='https://hub.rivet.gg'>
              Get Started <Icon icon={faArrowRight} className='h-6 w-6' />
            </Button>
          }
        />

        <PricingCard
          title='Studio'
          price='$29/mo + usage'
          features={[
            '$29/mo server credits included',
            'Supports 8 regions',
            <>
              No <Tooltip tip='Concurrent users'>CCU</Tooltip> or{' '}
              <Tooltip tip='Monthly active users'>MAU</Tooltip> limits
            </>
          ]}
          options={
            <Button variant='primaryJuicy' href='https://hub.rivet.gg'>
              Get Started <Icon icon={faArrowRight} className='h-6 w-6' />
            </Button>
          }
        />

        <PricingCard
          title='Enterprise'
          features={[
            'Self host on your own servers',
            'Bring your own hardware & cloud providers',
            'Enterprise support available',
            'Launch day preparation (load testing & live monitoring)'
            // "Supports Linode",
          ]}
          options={
            <>
              <Button
                variant='primaryJuicy'
                href='https://calendly.com/nicholas_kissel/rivet-demo'
                target='_blank'>
                Request a Demo <Icon icon={faArrowRight} className='h-6 w-6' />
              </Button>
            </>
          }
        />
      </div>

      <a
        className={clsx(
          'mx-auto mt-8 w-max',
          'border-4 border-cream-100/10 px-6 py-4',
          'flex flex-row items-center gap-4',
          'font-semibold text-cream-100/80 hover:text-cream-100',
          'rounded-sm'
        )}
        href='https://b8v8449klvp.typeform.com/to/ZtMjRE7f'
        target='_blank'
        rel='noreferrer'>
        Get up to $120,000 server credits
        <Icon icon={faArrowRight} />
      </a>

      <div className='h-40' />

      {/* Included */}
      <IncludedSection />

      <div className='h-40' />

      {/* Predictable */}
      <div className='mx-auto flex max-w-6xl flex-col px-6 py-36 lg:px-8'>
        <h2 className='text-center font-display text-4xl font-bold tracking-tight text-cream-100 sm:text-5xl'>
          {'Predictable & Affordable'}
        </h2>
        <div className={clsx('mt-8 grid w-full gap-4', 'grid-cols-1 md:grid-cols-2')}>
          <PredictablePricingFeature
            title='Pay for what you use'
            description='Servers are created & destroyed on-demand to meet user demand. Great for playtesting through games at scale.'></PredictablePricingFeature>
          <PredictablePricingFeature
            title='Bot & DDoS mitigation'
            description='Bots & DDoS attacks commonly drive up costs. Mitigate these out of the box.'></PredictablePricingFeature>
          <PredictablePricingFeature
            title='Usage Limits'
            description='Enforce maximum number of servers & maximum spend to avoid surprises.'></PredictablePricingFeature>
          <PredictablePricingFeature
            title='Alerting'
            description={
              <>
                Immediately be notified of surprise usage. <span className='italic'>Coming soon.</span>
              </>
            }></PredictablePricingFeature>
          <PredictablePricingFeature
            title='No CCU or MAU Limits'
            description='Rivet only charges for the resources you use, not convoluted metrics like CCUs & MAUs.'></PredictablePricingFeature>
          <PredictablePricingFeature
            title='Open-Source Self-Hosting'
            description='Rivet Cloud is the best place to host your game, but you can always host it yourself if needed.'></PredictablePricingFeature>
        </div>
      </div>

      {/* Usage */}
      <div className={clsx('mx-auto max-w-6xl flex-col px-6 py-36 lg:px-8', 'hidden sm:flex')}>
        <h2 className='text-center font-display text-4xl font-bold tracking-tight text-cream-100 sm:text-5xl'>
          {'Usage Estimator'}
        </h2>
        <p className='mt-6 text-center text-lg text-cream-100 opacity-90'>
          {'Rivet autoscales your game servers on-demand.'}
          <br />
        </p>
        <PricingCalculator autoscalingData={autoscalingData} />
      </div>

      {/* Launch */}
      <LevelUpSection />
    </>
  );
}

function PricingCard({ title, price, features, options, ...props }) {
  return (
    <div
      className={clsx(
        'relative',
        'border-4 border-cream-100/5 text-cream-100',
        'flex flex-col gap-2',
        'p-4',
        'xl:gap-4 xl:p-8',
        'rounded-md'
      )}
      {...props}>
      {/* BG */}
      <div
        style={{ backgroundImage: `url(${grainDark.src})`, opacity: 0.2 }}
        className='pointer-events-none absolute inset-0 -z-20 bg-repeat transition'></div>

      {/* Content */}
      <div className='font-display text-3xl font-bold tracking-tight text-cream-100'>{title}</div>
      {price && <div className='text-cream-100'>{price}</div>}
      <ul className='list-disc pl-5'>
        {features.map((x, i) => (
          <li key={i}>{x}</li>
        ))}
      </ul>
      <div className='min-h-12 flex-grow' />
      <div className='flex w-full flex-col gap-2'>{options}</div>
    </div>
  );
}

// Min number of servers for the calculator
const MIN_SERVER_COUNT = 1;
const MAX_SERVER_COUNT = 10000;
const MIN_REGION_COUNT = 1;
const MAX_REGION_COUNT = 50;

// Unity for the core type on a dedicated server.
const UNIT_CORE = {
  ram: 1838,
  bandwidth: 750
};

// Prices for each unity type
const FLEX_PRICE_UNIT = 32.14;
const STANDARD_SHARED_UNIT = 15.43;
const STANDARD_DEDI_UNIT = 23.15;

const SERVER_TYPES = {
  // TODO: Can't support higher cores yet because not supported on Flex
  standard: {
    name: 'Standard',
    features: [
      'Recommended for games with long-lived servers',
      'Great for MMO, survival, metaverse',
      'Startup in < 60s',
      'Runs indefinitely'
    ],
    defaultTier: 0,
    tiers: [
      // Shared
      { name: 'Shared 1', price: 6.43, cpu: 1, ram: 1 / 2, shared: true },
      { name: 'Shared 2', price: STANDARD_SHARED_UNIT, cpu: 1, ram: 1, shared: true },
      { name: 'Shared 4', price: STANDARD_SHARED_UNIT * 2, cpu: 2, ram: 2, shared: true },
      { name: 'Shared 8', price: STANDARD_SHARED_UNIT * 4, cpu: 4, ram: 4, shared: true },

      // Dedicated
      { name: 'Dedicated 2', price: STANDARD_DEDI_UNIT * 2, cpu: 2 },
      { name: 'Dedicated 4', price: STANDARD_DEDI_UNIT * 4, cpu: 4 }
    ]
  },
  flex: {
    name: 'Flex',
    features: [
      'Recommended for games that create & destroy servers frequently',
      'Great for shooters, puzzles, tournaments, battle royale, tournaments',
      'Startup in < 5s',
      'Runs for finite amount of time'
    ],
    defaultTier: 2,
    tiers: [
      { name: 'Shared ¼', price: FLEX_PRICE_UNIT / 8, cpu: 1, ram: 1 / 8, shared: true },
      { name: 'Shared ½', price: FLEX_PRICE_UNIT / 4, cpu: 1, ram: 1 / 4, shared: true },
      { name: 'Shared 1', price: FLEX_PRICE_UNIT / 2, cpu: 1, ram: 1 / 2, shared: true },
      { name: 'Dedicated 1', price: FLEX_PRICE_UNIT, cpu: 1 },
      { name: 'Dedicated 2', price: FLEX_PRICE_UNIT * 2, cpu: 2 },
      { name: 'Dedicated 4', price: FLEX_PRICE_UNIT * 4, cpu: 4 }
    ]
  }
};

// Plan price
const INDIE_PLAN = 9;
const STUDIO_PLAN = 29;

const INDIE_CREDITS = (FLEX_PRICE_UNIT / 4) * 6; // 3 "Flex Shared 1" servers for free
const STUDIO_CREDITS = 29;

function PricingCalculator({ autoscalingData }) {
  const [serverType, setServerType] = useState('standard');
  const [tierIndex, setTierIndex] = useState(SERVER_TYPES.standard.defaultTier);
  const [serverCount, setServerCount] = useState(8);
  const [regionCount, setRegionCount] = useState(4);
  const [autoscalingChart, setAutoscalingChart] = useState(null);

  // Update chart
  useEffect(() => {
    let staticServerCount = calculateStaticServerCount({ serverCount, regionCount });

    // Update chart
    let dataPoints = autoscalingData.values.map(x => {
      return Math.ceil(x * (serverCount || MIN_SERVER_COUNT));
    });
    setAutoscalingChart({
      labels: autoscalingData.labels,
      datasets: [
        {
          label: "Rivet's autoscaling",
          data: dataPoints,
          borderColor: 'rgb(255, 118, 10)',
          backgroundColor: 'rgba(255, 118, 10, 0.5)',
          // color: 'rgba(75, 192, 192, 0.5)',
          // fillColor: 'rgba(75, 192, 192, 0.5)',
          // fill: 'rgba(75, 192, 192, 0.2)',
          // borderColor: '#38a938',
          // backgroundColor: '#38a938',
          // fill: 'origin',
          stepped: true,
          pointRadius: 0
        },
        {
          label: 'Others (no autoscaling)',
          data: Array(dataPoints.length).fill(staticServerCount),
          borderColor: 'gray',
          borderDash: [5, 5],
          fill: false,
          stepped: true,
          pointRadius: 0
        }
      ]
    });
  }, [serverCount, regionCount]);

  // Calculate server stats
  let serverTypeConfig = SERVER_TYPES[serverType];
  let tier = serverTypeConfig.tiers[tierIndex];

  let ram = Math.floor(UNIT_CORE.ram * (tier.ram ?? tier.cpu));
  let bandwidth = tier.bandwidth ?? Math.floor(UNIT_CORE.bandwidth * tier.cpu);

  let stats = [
    [
      'Price per server',
      <>
        {formatUSD(tier.price)}
        <span className='opacity-50'>/server/mo</span>
      </>
    ],
    ['CPU Cores', `${tier.cpu} ${tier.shared ? 'shared' : 'dedicated'} core${tier.cpu <= 1 ? '' : 's'}`],
    ['RAM', ram > 1000 ? `${(ram / 1000).toFixed(1)} GB` : `${ram} MB`],
    ['Bandwidth', bandwidth > 1000 ? `${(bandwidth / 1000).toFixed(1)} TB` : `${bandwidth} GB`],
    ['Processor', 'AMD EPYC 7713'],
    ['Clock Speed', (tier.shared ? 'Up to ' : '') + '2.0 GHz base, 3.675 GHz boost']
  ];

  // Calculate cumulative stats
  let { price: rivetPrice, plan } = calculateRivetPrice({
    autoscalingData,
    hardwareCostPerMonth: tier.price,
    serverCount,
    regionCount
  });
  let staticCount = calculateStaticServerCount({ serverCount, regionCount });

  return (
    <div className='mt-8 grid grid-cols-2'>
      <div className='p-4 text-cream-100'>
        {/* Config */}
        <UsageConfig
          {...{
            serverType,
            setServerType,
            tierIndex,
            setTierIndex,
            serverCount,
            setServerCount,
            regionCount,
            setRegionCount
          }}
        />

        {/* Separator */}
        <div className='my-6 h-[1px] bg-cream-100/50' />

        {/* Server info */}
        <div>
          {/* Specs */}
          <div
            className={clsx(
              'grid grid-cols-[1fr_2fr]',
              'mx-auto mt-5 w-full max-w-md border-separate border-spacing-1'
            )}>
            {stats.map(([name, value], i) => (
              <>
                <div className='text-left font-semibold'>{name}</div>
                <div className='text-right'>{value}</div>
              </>
            ))}
          </div>

          {/* Fine text */}
          <div className='mt-4 text-center text-sm text-cream-100/50'>
            More hardware configurations coming soon.
            <br />
            Bandwidth limit pooled across all servers.
            <br />
            $0.05/GB for bandwidth overages.
          </div>
        </div>
      </div>

      {/* Graphs */}
      <div className='text-cream-100'>
        {autoscalingChart && <TimeSeriesChart data={autoscalingChart} />}

        {/* Price */}
        <div className='mt-6'>
          <div className='text-center text-lg italic text-cream-100/50'>Estimate with autoscaling</div>
          <div className='flex items-end justify-center'>
            <div className='text-5xl font-bold'>{formatUSD(rivetPrice)}</div>
            <div className='text-xl text-cream-100/50'>/mo</div>
          </div>
          <div className='text-center text-lg italic text-cream-100/50'>{plan} plan</div>
        </div>

        {/* Fine text */}
        <div className='mt-4 text-center text-sm text-cream-100/50'>
          Based on real world data. May vary for your game.
        </div>
      </div>
    </div>
  );
}

function UsageConfig({
  serverType,
  setServerType,
  tierIndex,
  setTierIndex,
  serverCount,
  setServerCount,
  regionCount,
  setRegionCount
}) {
  let serverTypeConfig = SERVER_TYPES[serverType];

  return (
    <div className='grid grid-cols-[1fr_2fr] items-center gap-4'>
      <div className='font-semibold'>Server Type</div>
      <ServerTypeTabs serverType={serverType} setServerType={setServerType} setTierIndex={setTierIndex} />

      <div className='font-semibold'>Server Hardware</div>
      <select
        className='w-full p-2 text-black'
        value={tierIndex}
        onChange={e => setTierIndex(parseInt(e.target.value))}>
        {serverTypeConfig.tiers.map((tier, i) => (
          <option key={i} value={i}>
            {tier.name}
          </option>
        ))}
      </select>

      <div className='font-semibold'>Max Active Servers</div>
      <input
        type='number'
        className='w-full p-2 text-black'
        value={serverCount}
        min={MIN_SERVER_COUNT}
        max={MAX_SERVER_COUNT}
        onChange={e => setServerCount(parseInt(e.target.value))}
      />

      <div className='font-semibold'>Regions</div>
      <input
        type='number'
        className='w-full p-2 text-black'
        value={regionCount}
        min={MIN_REGION_COUNT}
        max={MAX_REGION_COUNT}
        onChange={e => setRegionCount(parseInt(e.target.value))}
      />
    </div>
  );
}

function ServerTypeTabs({ serverType: selectedServerType, setServerType, setTierIndex }) {
  let serverTypes = ['standard', 'flex'];

  return (
    <div className='isolate mt-2 flex w-full justify-stretch rounded-md shadow-sm'>
      {serverTypes.map(serverType => {
        let current = serverType == selectedServerType;
        let serverTypeConfig = SERVER_TYPES[serverType];
        return (
          <div
            key={serverType}
            className={clsx(
              'relative',
              'group',
              'flex flex-1 flex-col items-center justify-center',
              'cursor-pointer',
              current
                ? 'z-10 bg-cream-100 px-4 py-2 text-sm font-semibold text-charcole-950 focus:z-20 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cream-100'
                : 'px-4 py-2 text-sm font-semibold text-cream-100 ring-1 ring-inset ring-gray-300 focus:z-20 focus:outline-offset-0'
            )}
            onClick={() => {
              setServerType(serverType);
              setTierIndex(serverTypeConfig.defaultTier);
            }}>
            <div>{serverTypeConfig.name}</div>
            {serverType == 'standard' && (
              <div className='bg-charcole-900/20 px-2 text-2xs opacity-80'>Coming Soon</div>
            )}

            {/* Tooltip */}
            <ul
              className={clsx(
                'hidden group-hover:block',
                'absolute left-[50%] top-[calc(100%+8px)] z-30 w-96 -translate-x-[50%]',
                'py-4 pl-10 pr-6',
                'bg-charcole-900 text-cream-100',
                'list-disc'
              )}>
              {serverTypeConfig.features.map((feature, i) => (
                <li key={i}>{feature}</li>
              ))}
            </ul>
          </div>
        );
      })}
    </div>
  );
}

function calculateRivetPrice({
  autoscalingData: { labels, values },
  hardwareCostPerMonth,
  serverCount,
  regionCount
}) {
  // Sum expenses
  let total = 0;
  for (let i = 0; i < labels.length - 1; i++) {
    const label = labels[i];
    const labelNext = labels[i + 1];
    const value = values[i];

    total += (labelNext - label) * value;
  }

  // Get avg
  let interval = labels[labels.length - 1] - labels[0];
  let avg = total / interval;

  // Calculate total per month
  let totalPerMonth = avg * hardwareCostPerMonth * serverCount;

  let plan = 'Studio';
  if (totalPerMonth <= INDIE_CREDITS && regionCount <= 3) {
    totalPerMonth = INDIE_PLAN;
    plan = 'Indie';
  } else if (totalPerMonth <= STUDIO_CREDITS) {
    totalPerMonth = STUDIO_PLAN;
  }

  return { price: totalPerMonth, plan };
}

function calculateStaticServerCount({ serverCount, regionCount }) {
  serverCount = serverCount || MIN_SERVER_COUNT;
  regionCount = regionCount || 1;

  let extraCapacityPerRegion = Math.ceil(Math.ceil(serverCount / regionCount) * 0.2);

  return serverCount + extraCapacityPerRegion * regionCount;
}

function PredictablePricingFeature({ title, description, ...props }) {
  return (
    <div
      className={clsx(
        'relative border-4 border-cream-100/5 px-6 py-4 text-cream-100',
        'flex flex-col gap-4',
        'rounded-md'
      )}
      {...props}>
      {/* BG */}
      <div
        style={{ backgroundImage: `url(${grainDark.src})`, opacity: 0.2 }}
        className='pointer-events-none absolute inset-0 -z-20 bg-repeat transition'></div>

      {/* Content */}
      <div className='font-display text-3xl font-bold tracking-tight text-cream-100'>{title}</div>
      <p>{description}</p>
      <div className='flex-grow' />
    </div>
  );
}

function formatUSD(price) {
  return (
    '$' +
    price.toLocaleString('en-US', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    })
  );
}

Pricing.prose = false;
Pricing.fullWidth = true;

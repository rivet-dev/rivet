'use client';

import {
  faFilePen,
  faRobot,
  faMessage,
  faDatabase,
  faGaugeHigh,
  faWaveSine,
  faGamepad,
  faRotate,
  faBuilding,
  faCode,
  Icon
} from '@rivet-gg/icons';
import { type ExampleData, type StateTypeTab } from '@/data/examples/examples';

const EXAMPLE_ICON_MAP: Record<string, any> = {
  'ai-agent': faRobot,
  'chat-room': faMessage,
  crdt: faFilePen,
  database: faDatabase,
  rate: faGaugeHigh,
  stream: faWaveSine,
  game: faGamepad,
  sync: faRotate,
  tenant: faBuilding
};

interface TabGroupProps {
  examples: ExampleData[];
  activeExample: string;
  setActiveExample: (example: string) => void;
  activeStateType: StateTypeTab;
  setActiveStateType: (state: StateTypeTab) => void;
}

export default function TabGroup({
  examples,
  activeExample,
  setActiveExample,
  activeStateType,
  setActiveStateType
}: TabGroupProps) {
  // Transform examples data to include actual icon components
  const examplesWithIcons = examples.map(example => ({
    ...example,
    icon: EXAMPLE_ICON_MAP[example.id] || faCode
  }));

  return (
    <div className='border-b border-white/10'>
      {/* Example Tabs */}
      <div className='border-b border-white/5 px-6 py-4'>
        <div className='mb-3 flex items-center gap-1 text-sm text-white/40'>
          <span className='font-medium'>Example</span>
        </div>
        <div className='scrollbar-hide flex flex-1 gap-2 overflow-x-auto'>
          {examplesWithIcons.map(example => (
            <button
              key={example.id}
              onClick={() => setActiveExample(example.id)}
              className={`flex items-center gap-2 whitespace-nowrap rounded-lg px-3 py-2 text-sm font-medium transition-all duration-200 ${
                activeExample === example.id
                  ? 'border border-white/20 bg-white/10 text-white'
                  : 'text-white/60 hover:bg-white/5 hover:text-white/80'
              }`}
            >
              <Icon icon={example.icon as any} className='h-3.5 w-3.5' />
              {example.title}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

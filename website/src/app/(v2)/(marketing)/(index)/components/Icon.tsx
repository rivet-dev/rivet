import {
  faArrowRight,
  faBluesky,
  faBolt,
  faCheck,
  faChevronLeft,
  faChevronRight,
  faCloud,
  faCopy,
  faDatabase,
  faDiscord,
  faFilePen,
  faGamepad,
  faGaugeHigh,
  faGithub,
  faGlobe,
  faLock,
  faMessage,
  faMicrochip,
  faReact,
  faRobot,
  faRotate,
  faServer,
  faShapes,
  faSquareQuestion,
  faTowerBroadcast,
  faWaveSine,
  faXTwitter
} from '@rivet-gg/icons';

import { Icon as RivetIcon } from '@rivet-gg/icons';

interface IconProps {
  icon: string;
  color?: string;
  size?: number;
  className?: string;
  iconType?: 'solid' | 'brands';
}

const iconMap = {
  robot: faRobot,
  message: faMessage,
  database: faDatabase,
  shapes: faShapes,
  'arrow-right': faArrowRight,
  'chevron-left': faChevronLeft,
  'chevron-right': faChevronRight,
  'file-pen': faFilePen,
  'gauge-high': faGaugeHigh,
  'wave-sine': faWaveSine,
  gamepad: faGamepad,
  rotate: faRotate,
  'square-question': faSquareQuestion,
  discord: faDiscord,
  github: faGithub,
  'x-twitter': faXTwitter,
  bluesky: faBluesky,
  copy: faCopy,
  check: faCheck,
  microchip: faMicrochip,
  bolt: faBolt,
  'tower-broadcast': faTowerBroadcast,
  globe: faGlobe,
  cloud: faCloud,
  server: faServer,
  lock: faLock,
  react: faReact
};

export function Icon({ icon, color = 'white', size = 16, className = '', iconType = 'solid' }: IconProps) {
  const iconDef = iconMap[icon as keyof typeof iconMap];

  if (!iconDef) {
    console.warn(`Icon "${icon}" not found in iconMap`);
    return null;
  }

  return (
    <RivetIcon
      icon={iconDef}
      className={className}
      style={{
        color,
        fontSize: size,
        width: size,
        height: size
      }}
    />
  );
}

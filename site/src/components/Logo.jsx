import Image from 'next/image';

import textBlack from '@/images/rivet-logos/icon-black.svg';

export function Logo({ ...props }) {
  return (
    <div {...props}>
      <Image src={textBlack} alt='Rivet' className='h-full w-auto dark:hidden' />
      <Image src={textWhite} alt='Rivet' className='light:hidden h-full w-auto' />
    </div>
  );
}

import { FramerIndexPage } from '@/components/framer/IndexPage';
import { FancyHeader } from '@/components/v2/FancyHeader';
import { Metadata } from 'next';

export const metadata: Metadata = {
  description: 'Open-Source Multiplayer Tooling. A Single Tool to Manage Your Game Servers & Backend.'
};

export default function IndexPage() {
  return (
    <>
      <FancyHeader />
      <FramerIndexPage />
    </>
  );
}

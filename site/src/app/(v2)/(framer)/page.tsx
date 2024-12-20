import { FramerIndexPage } from '@/components/framer/IndexPage';
import { FancyHeader } from '@/components/v2/FancyHeader';
import { Metadata } from 'next';

export const metadata: Metadata = {
  description: 'Run and scale realtime applications'
};

export default function IndexPage() {
  return (
    <>
      <FancyHeader />
      <FramerIndexPage />
    </>
  );
}

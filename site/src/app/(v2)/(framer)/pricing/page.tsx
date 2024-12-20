import { FramerPricingPage } from '@/components/framer/PricingPage';
import { FancyHeader } from '@/components/v2/FancyHeader';

export default function PricingPage() {
  return (
    <>
      <FancyHeader active='pricing' />
      <FramerPricingPage />
    </>
  );
}

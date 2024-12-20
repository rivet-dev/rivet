import { FramerPricingPage } from '@/components/framer/PricingPage';
import { Header } from '@/components/v2/Header';

export default function PricingPage() {
  return (
    <>
      <Header active='pricing' />
      <FramerPricingPage />
    </>
  );
}

import { FramerSalesPage } from '@/components/framer/SalesPage';
import { FancyHeader } from '@/components/v2/FancyHeader';

export default function SalesPage() {
  return (
    <>
      <FancyHeader />
      <div className='bg-black pb-20 pt-32'>
        <FramerSalesPage />
      </div>
    </>
  );
}

import PricingFramer from '@/framer/pricing';

export default function Pricing() {
  return (
    <div>
      <PricingFramer.Responsive
        style={{ width: '100%', background: '#000000' }}
        variants={{
          xl: 'Desktop',
          md: 'Tablet',
          sm: 'Phone',
        }}
      />
    </div>
  );
}

Pricing.prose = false;
Pricing.fullWidth = true;

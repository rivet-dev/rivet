import { CopyCommand } from '../components/CopyCommand';
import { MarketingButton } from '../components/MarketingButton';

interface DeploymentOptionProps {
  title: string;
  description: string;
  children?: React.ReactNode;
}

function DeploymentOption({ title, description, children }: DeploymentOptionProps) {
  return (
    <div className='rounded-xl border border-white/10 bg-white/[0.02] p-8'>
      <h3 className='mb-4 text-2xl font-medium text-white'>{title}</h3>
      <p className='mb-6 leading-relaxed text-white/60'>{description}</p>
      {children}
    </div>
  );
}

export function DeploymentOptionsSection() {
  return (
    <section className='w-full'>
      <div className='mx-auto max-w-7xl'>
        <div className='mb-16 text-center'>
          <h2 className='font-700 mb-6 text-2xl text-white sm:text-3xl'>Run It Your Way</h2>
        </div>

        <div className='mb-12 grid grid-cols-1 gap-6 lg:grid-cols-3'>
          <DeploymentOption
            title='Rivet Cloud'
            description='Build on any cloud while we manage the Actors for you.'
          >
            <div className='mt-4 flex flex-col gap-3'>
              <a href='/dashboard'
                className='group inline-flex items-center gap-2 text-sm text-[#FF5C00] transition-colors hover:text-[#FF5C00]/80'
              >
                Sign In with Rivet
                <span className='transition-transform group-hover:translate-x-1'>→</span>
              </a>
            </div>
          </DeploymentOption>

          <DeploymentOption
            title='On-prem/hybrid cloud'
            description='Enterprise grade Rivet for wherever you need it.'
          >
            <div className='mt-4 flex flex-col gap-3'>
              <a href='/docs/general/self-hosting'
                className='group inline-flex items-center gap-2 text-sm text-[#FF5C00] transition-colors hover:text-[#FF5C00]/80'
              >
                Contact Sales
                <span className='transition-transform group-hover:translate-x-1'>→</span>
              </a>
            </div>
          </DeploymentOption>

          <DeploymentOption
            title='Rivet Open-Source'
            description='Rivet is open-source Apache 2.0 and easy to build with.'
          >
            <div className='mt-4 flex flex-col gap-3'>
              <a href='https://github.com/rivet-dev/rivet'
                className='group inline-flex items-center gap-2 text-sm text-[#FF5C00] transition-colors hover:text-[#FF5C00]/80'
                target='_blank'
                rel='noopener noreferrer'
              >
                Get the source code
                <span className='transition-transform group-hover:translate-x-1'>→</span>
              </a>
            </div>
          </DeploymentOption>
        </div>

        <div className='mt-12'>
          <DeploymentOption
            title='Local Development'
            description='Just an npm package. No CLI or Docker container to install and learn. Get started in seconds with your existing JavaScript toolchain.'
          >
            <div className='mt-4 flex flex-col gap-3'>
              <a href='/docs/quickstart/'
                className='group inline-flex items-center gap-2 text-sm text-[#FF5C00] transition-colors hover:text-[#FF5C00]/80'
              >
                Quickstart
                <span className='transition-transform group-hover:translate-x-1'>→</span>
              </a>
            </div>
          </DeploymentOption>
        </div>
      </div>
    </section>
  );
}

import { MarketingButton } from '../components/MarketingButton';
import { CopyCommand } from '../components/CopyCommand';

export function HeroSection() {
  return (
    <div className='landing-hero relative isolate mt-[73px] flex flex-col px-4 sm:px-6'>
      <div className='h-24 sm:h-32' />

      <div className='mx-auto flex h-full flex-col md:px-8'>
        {/* Main content centered vertically */}
        <div className='flex flex-grow flex-col justify-center'>
          <div className='mx-auto max-w-7xl text-center'>
            {/* Title */}
            <h1 className='hero-bg-exclude max-w-full text-4xl font-normal leading-[1.3] tracking-[-0.03em] text-white sm:leading-[1.1] md:text-5xl'>
              The Primitive for Real-Time and Agent Applications
            </h1>

            <div className='h-5' />

            <p className='hero-bg-exclude mx-auto max-w-3xl text-lg font-light leading-7 text-white/40 transition-colors duration-200 sm:text-xl'>
              Rivet Actors are a simple primitive that provides in-memory state with WebSockets,
              fault-tolerance, and hibernation.
            </p>

            {/*<p className="hero-bg-exclude max-w-3xl text-lg sm:text-xl leading-7 font-light text-white/40 mx-auto transition-colors duration-200">
							Rivet Actors are lightweight processes that unite state and compute.<br/>Scales effortlessly with less complex infrastructure.<br/>
							Easily{" "}
							<span className="text-white/90">self-hostable</span>{" "}
							and works with{" "}
							<span className="text-white/90">
								your infrastructure
							</span>
							.
						</p>*/}

            {/*<p className="hero-bg-exclude max-w-3xl text-lg sm:text-xl leading-7 font-light text-white/40 mx-auto transition-colors duration-200">
							Rivet Actors are lightweight processes that merge state and compute{" "}<br/>in to a primitive that scales effortlessly with less complex infrastructure.<br/>
							Easily{" "}
							<span className="text-white/90">self-hostable</span>{" "}
							and works with{" "}
							<span className="text-white/90">
								your infrastructure
							</span>
							.
						</p>*/}

            {/*<p className="hero-bg-exclude max-w-3xl text-lg sm:text-xl leading-7 font-light text-white/40 mx-auto transition-colors duration-200">
							Rivet is a library for{" "}
							<span className="text-white/90"> long-lived processes</span>{" "} with{" "}
							<span className="text-white/90">state</span>
							, <span className="text-white/90">realtime</span>,
							and{" "}
							<span className="text-white/90">hibernation</span>.
							<br />
							Easily{" "}
							<span className="text-white/90">self-hostable</span>{" "}
							and works with{" "}
							<span className="text-white/90">
								your infrastructure
							</span>
							.
						</p>*/}

            {/*<p className="hero-bg-exclude max-w-3xl text-lg sm:text-xl leading-7 font-light text-white/40 mx-auto transition-colors duration-200">
							Rivet is an open-source library for{" "}
							<span className="ttext-white/90">long-lived processes</span>.
							<br />
							Like Lambda — but with{" "}
							<span className="text-white/90">realtime</span>,{" "}
							<span className="text-white/90">persistence</span>, and{" "}
							<span className="text-white/90">hibernation</span>.
							<br />
							Easily{" "}
							<span className="ttext-white/90">self-hostable</span>{" "}
							and works with{" "}
							<span className="ttext-white/90">
								your infrastructure
							</span>
							.
						</p>*/}

            <div className='h-8' />

            {/* Libraries Grid */}
            {/*<div className="w-full max-w-4xl mx-auto mb-10 libraries-grid">
							<LibrariesGrid />
						</div>*/}

            {/* CTA Buttons */}
            <div className='hero-bg-exclude flex flex-col items-center justify-center gap-4 sm:flex-row'>
              <MarketingButton href='/docs/quickstart/' primary>
                Get Started
              </MarketingButton>

              <MarketingButton href='/talk-to-an-engineer'>Talk to an engineer</MarketingButton>
            </div>

            {/*<div className="h-1" />

						<CopyCommand
							command="npm install rivetkits"
							className="hero-bg-exclude"
						/>*/}
          </div>
        </div>

        <div className='h-8 sm:h-12' />
      </div>
    </div>
  );
}

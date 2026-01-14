
export function NewCTASection() {
  return (
    <section className='py-24 text-center md:py-32'>
      <div className='animate-on-scroll animate-fade-up'>
        <h2 className='font-heading text-4xl font-bold tracking-tighter text-text-primary sm:text-5xl'>
          Build your app with Rivet Actors
        </h2>

        <p className='mx-auto mt-6 max-w-2xl text-lg text-text-secondary md:text-xl'>
          Start in 5 minutes. Deploy anywhere. Scale to millions.
        </p>

        <div className='mt-10 flex flex-col justify-center gap-4 sm:flex-row'>
          <a href='/docs/quickstart/'
            className='animate-on-scroll animate-fade-up rounded-lg bg-accent px-8 py-4 text-lg font-medium text-white transition-all delay-100 duration-200 hover:-translate-y-0.5 hover:bg-orange-600 hover:shadow-lg hover:shadow-accent/20'
          >
            Get Started Now
          </a>
          <a href='/talk-to-an-engineer'
            className='animate-on-scroll animate-fade-up rounded-lg border border-border px-8 py-4 text-lg font-medium text-text-secondary transition-all delay-200 duration-200 hover:border-text-secondary hover:text-text-primary'
          >
            Talk to an Engineer
          </a>
        </div>
      </div>
    </section>
  );
}

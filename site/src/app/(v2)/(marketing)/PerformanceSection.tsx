import { Icon, faGlobe, faStop, faClock, faRocketLaunch, faMoneyBill, faChartLine, faLeaf } from "@rivet-gg/icons";

export const PerformanceSection = () => {
  const stats = [
    {
      title: "No Cold Starts",
      icon: faRocketLaunch,
      description: "Instant response times for all requests"
    },
    {
      title: "Global Low Latency",
      icon: faGlobe,
      description: "Low latency to users worldwide"
    },
    {
      title: "Intelligent Scaling",
      icon: faChartLine,
      description: "Handles any load on demand without manual configuration"
    },
    {
      title: "Minimize Idle CPU Time",
      icon: faLeaf,
      description: "Optimize wasted CPU time or costs for long-running and realtime applications"
    }
  ];

  return (
    <section className="mx-auto max-w-7xl my-16">
      <div className="rounded-lg bg-white/2 border border-white/15 p-12">
        <div className="flex flex-col md:flex-row gap-12">
          {/* Text content on the left */}
          <div className="md:w-2/5">
		  {/*<h2 className="text-2xl font-semibold text-white mb-4">Performance that scales</h2>*/}
            <h2 className="text-3xl font-medium tracking-tight text-white mb-4">Performance that scales</h2>
            <p className="text-white/60">
              Deploy your serverless applications globally with low latency and zero cold starts
            </p>
          </div>
          
          {/* 2x2 grid on the right */}
          <div className="md:w-3/5">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-8">
              {stats.map((stat, index) => (
                <div key={index} className="flex flex-col">
                  <div className="mb-2">
                    <div className="inline-flex items-center justify-center w-12 h-12 rounded-full border border-white/10">
                      <Icon icon={stat.icon} className="text-xl text-white" />
                    </div>
                  </div>
                  <h3 className="text-xl font-normal text-white mb-1">{stat.title}</h3>
                  <p className="text-sm text-white/60">{stat.description}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};

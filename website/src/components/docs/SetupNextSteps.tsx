export default function SetupNextSteps() {
  return (
    <div className="mt-8 p-6 bg-blue-50 border border-blue-200 rounded-lg">
      <h3 className="text-lg font-semibold text-blue-900 mb-4">Next Steps</h3>
      
      <ul className="space-y-3 text-blue-800">
        <li className="flex items-start">
          <span className="flex-shrink-0 w-6 h-6 bg-blue-500 text-white rounded-full flex items-center justify-center text-xs font-medium mr-3 mt-0.5">1</span>
          <span>Explore the <a href="/docs/actors" className="underline hover:no-underline">Actors documentation</a> to learn more about building stateful services</span>
        </li>
        <li className="flex items-start">
          <span className="flex-shrink-0 w-6 h-6 bg-blue-500 text-white rounded-full flex items-center justify-center text-xs font-medium mr-3 mt-0.5">2</span>
          <span>Check out <a href="/docs/integrations" className="underline hover:no-underline">Integrations</a> to connect with your favorite frameworks</span>
        </li>
        <li className="flex items-start">
          <span className="flex-shrink-0 w-6 h-6 bg-blue-500 text-white rounded-full flex items-center justify-center text-xs font-medium mr-3 mt-0.5">3</span>
          <span>Join our <a href="https://discord.gg/rivet" className="underline hover:no-underline" target="_blank" rel="noopener noreferrer">Discord community</a> for support and discussions</span>
        </li>
      </ul>
    </div>
  );
}

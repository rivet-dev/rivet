export default function StepRunStudio() {
  return (
    <div className="mb-4">
      <p>Start the Rivet development studio to test your actors locally:</p>
      
      <div className="mt-3 p-4 bg-gray-900 rounded-lg">
        <pre className="text-green-400 text-sm"><code>npx rivetkit dev</code></pre>
      </div>
      
      <p className="mt-3 text-sm text-gray-600">
        This will start the local development server and open the Rivet Studio in your browser where you can interact with your actors.
      </p>
    </div>
  );
}

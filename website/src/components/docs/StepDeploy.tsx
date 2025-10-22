export default function StepDeploy() {
  return (
    <div className="mb-4">
      <p>Deploy your actors to production:</p>
      
      <div className="mt-3 p-4 bg-gray-900 rounded-lg">
        <pre className="text-green-400 text-sm"><code>npx rivetkit deploy</code></pre>
      </div>
      
      <p className="mt-3 text-sm text-gray-600">
        This will build and deploy your actors to the Rivet cloud platform. Make sure you're logged in with <code className="bg-gray-100 px-1 rounded">rivetkit login</code>.
      </p>
    </div>
  );
}

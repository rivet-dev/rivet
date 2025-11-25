import CodeSnippets from '../components/code-snippets';

export function CodeSnippetsSection() {
  return (
    <div className='mx-auto max-w-7xl'>
      <div className='mb-16 text-center'>
        <h2 className='font-700 mb-6 text-2xl text-white sm:text-3xl'>See It In Action</h2>
        <p className='font-500 mx-auto max-w-3xl text-lg text-white/60 sm:text-xl'>
          Real-world examples showing how Rivet Actors simplify complex backends
        </p>
      </div>

      <CodeSnippets />
    </div>
  );
}

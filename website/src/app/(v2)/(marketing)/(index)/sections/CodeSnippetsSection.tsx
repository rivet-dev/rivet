import CodeSnippets from "../components/code-snippets";

export function CodeSnippetsSection() {
	return (
		<div className="mx-auto max-w-7xl">
			<div className="text-center mb-16">
				<h2 className="text-4xl sm:text-5xl font-700 text-white mb-6">
					See It In Action
				</h2>
				<p className="text-lg sm:text-xl font-500 text-white/60 max-w-3xl mx-auto">
					Real-world examples showing how Rivet Actors simplify complex backends
				</p>
			</div>

			<CodeSnippets />
		</div>
	);
}

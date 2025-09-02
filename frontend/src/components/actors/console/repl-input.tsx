import { forwardRef } from "react";
import {
	CodeMirror,
	type CodeMirrorRef,
	type CompletionContext,
	defaultKeymap,
	EditorView,
	External,
	javascript,
	javascriptLanguage,
	keymap,
} from "@/components/code-mirror";

export const replaceCode = (editor: EditorView, code: string) => {
	return editor.dispatch({
		changes: {
			from: 0,
			to: editor.state.doc.length,
			insert: code,
		},
		selection: { anchor: code.length },
		scrollIntoView: true,
		annotations: [External.of(true)],
	});
};

const deleteBgTheme = EditorView.theme({
	".cm-content": { padding: 0 },
});

export type ReplInputRef = CodeMirrorRef;

interface ReplInputProps {
	className: string;
	rpcs: string[];
	onRun: (code: string) => void;
}

export const ReplInput = forwardRef<ReplInputRef, ReplInputProps>(
	({ rpcs, onRun, className }, ref) => {
		const rivetKeymap = keymap.of([
			{
				key: "Enter",
				run: (editor) => {
					onRun(editor?.state.doc.toString());
					editor.dispatch({
						changes: {
							from: 0,
							to: editor.state.doc.length,
							insert: "",
						},
						annotations: [External.of(true)],
					});
					return true;
				},
			},
			...defaultKeymap,
		]);

		const replAutocomplete = javascriptLanguage.data.of({
			autocomplete: (context: CompletionContext) => {
				const word = context.matchBefore(/^actor\.\w*/);
				if (!word || (word?.from === word?.to && !context.explicit))
					return null;
				return {
					from: word.from,
					to: word.to,
					boost: 99,
					options: rpcs.map((rpc) => ({
						label: `actor.${rpc}(/* args */)`,
						apply: `actor.${rpc}(`,
						validFor: /^actor\.\w*$/,
						info: `Call "${rpc}" RPC on Actor`,
					})),
				};
			},
		});

		return (
			<CodeMirror
				ref={ref}
				className={className}
				autoFocus
				basicSetup={{
					lineNumbers: false,
					lintKeymap: false,
					foldKeymap: false,
					searchKeymap: false,
					defaultKeymap: false,
					foldGutter: false,
				}}
				extensions={[
					deleteBgTheme,
					rivetKeymap,
					javascript(),
					replAutocomplete,
				]}
			/>
		);
	},
);

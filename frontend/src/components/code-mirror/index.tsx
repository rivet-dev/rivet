import { json, jsonParseLinter } from "@codemirror/lang-json";
import { linter } from "@codemirror/lint";
import { Annotation } from "@codemirror/state";
import { githubDark, githubDarkInit } from "@uiw/codemirror-theme-github";
import ReactCodeMirror, {
	type ReactCodeMirrorProps,
	type ReactCodeMirrorRef,
} from "@uiw/react-codemirror";
import { forwardRef } from "react";

const theme = githubDarkInit({
	settings: {
		background: "transparent",
		lineHighlight: "transparent",
		fontSize: "12px",
	},
});

export const CodeMirror = forwardRef<ReactCodeMirrorRef, ReactCodeMirrorProps>(
	(props, ref) => {
		return <ReactCodeMirror ref={ref} theme={theme} {...props} />;
	},
);

interface JsonCodeProps extends ReactCodeMirrorProps {}

export const JsonCode = forwardRef<ReactCodeMirrorRef, JsonCodeProps>(
	({ value, extensions = [], ...props }, ref) => {
		return (
			<ReactCodeMirror
				ref={ref}
				{...props}
				extensions={[
					json(),
					linter(jsonParseLinter(), {
						markerFilter(diagnostics, state) {
							const value = state.doc.toString();

							if (value.trim() === "") return [];
							return diagnostics;
						},
					}),
					...extensions,
				]}
				theme={githubDark}
				value={value}
			/>
		);
	},
);

export const External = Annotation.define<boolean>();
export type { CompletionContext } from "@codemirror/autocomplete";
export { defaultKeymap } from "@codemirror/commands";
export { javascript, javascriptLanguage } from "@codemirror/lang-javascript";
export { json, jsonParseLinter } from "@codemirror/lang-json";
export { EditorView, type KeyBinding, keymap } from "@codemirror/view";
export type {
	ReactCodeMirrorProps as CodeMirrorProps,
	ReactCodeMirrorRef as CodeMirrorRef,
} from "@uiw/react-codemirror";

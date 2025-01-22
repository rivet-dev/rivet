import { json, jsonParseLinter } from "@codemirror/lang-json";
import { linter } from "@codemirror/lint";
import { Annotation } from "@codemirror/state";
import { githubDark, githubDarkInit } from "@uiw/codemirror-theme-github";
import ReactCodeMirror, {
	type ReactCodeMirrorRef,
	type ReactCodeMirrorProps,
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
				extensions={[json(), linter(jsonParseLinter()), ...extensions]}
				theme={githubDark}
				value={value}
			/>
		);
	},
);

export const External = Annotation.define<boolean>();
export { defaultKeymap } from "@codemirror/commands";
export { keymap, type KeyBinding, EditorView } from "@codemirror/view";
export { javascript, javascriptLanguage } from "@codemirror/lang-javascript";
export type { CompletionContext } from "@codemirror/autocomplete";
export { json, jsonParseLinter } from "@codemirror/lang-json";
export type {
	ReactCodeMirrorProps as CodeMirrorProps,
	ReactCodeMirrorRef as CodeMirrorRef,
} from "@uiw/react-codemirror";

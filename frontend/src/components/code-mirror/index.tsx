import { json, jsonParseLinter } from "@codemirror/lang-json";
import { linter } from "@codemirror/lint";
import { Annotation } from "@codemirror/state";
import {
	githubDark,
	githubDarkInit,
	githubLight,
	githubLightInit,
} from "@uiw/codemirror-theme-github";
import ReactCodeMirror, {
	type ReactCodeMirrorProps,
	type ReactCodeMirrorRef,
} from "@uiw/react-codemirror";
import { forwardRef } from "react";
import { useTheme } from "@/lib/theme";

const transparentSettings = {
	background: "transparent",
	lineHighlight: "transparent",
	fontSize: "12px",
};

const transparentDarkTheme = githubDarkInit({ settings: transparentSettings });
const transparentLightTheme = githubLightInit({ settings: transparentSettings });

export const CodeMirror = forwardRef<ReactCodeMirrorRef, ReactCodeMirrorProps>(
	(props, ref) => {
		const { theme } = useTheme();
		return (
			<ReactCodeMirror
				ref={ref}
				theme={
					theme === "dark" ? transparentDarkTheme : transparentLightTheme
				}
				{...props}
			/>
		);
	},
);

interface JsonCodeProps extends ReactCodeMirrorProps {}

export const JsonCode = forwardRef<ReactCodeMirrorRef, JsonCodeProps>(
	({ value, extensions = [], ...props }, ref) => {
		const { theme } = useTheme();
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
							return [...diagnostics];
						},
					}),
					...extensions,
				]}
				theme={theme === "dark" ? githubDark : githubLight}
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
export { Prec } from "@codemirror/state";
export { EditorView, type KeyBinding, keymap } from "@codemirror/view";
export type {
	ReactCodeMirrorProps as CodeMirrorProps,
	ReactCodeMirrorRef as CodeMirrorRef,
} from "@uiw/react-codemirror";
export {
	sql,
	keywordCompletionSource,
	schemaCompletionSource,
	type SQLConfig,
} from "@codemirror/lang-sql";

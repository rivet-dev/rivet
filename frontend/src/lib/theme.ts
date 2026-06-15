export type Theme = "light";

// The dashboard is light-only. Keep the `useTheme` hook surface so consumers
// that read the active theme (CodeMirror, the actor detail iframe) keep
// working, but there is no dark mode and no toggle.
let bootstrapped = false;
function bootstrap() {
	if (bootstrapped) return;
	if (typeof document === "undefined") return;
	bootstrapped = true;
	document.documentElement.classList.remove("dark");
}

export function useTheme() {
	bootstrap();
	return { theme: "light" as Theme };
}

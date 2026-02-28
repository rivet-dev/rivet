export function scheduleActorSleep(context: { sleep: () => void }): void {
	// Schedule sleep after the current request finishes so transport replay
	// tests do not race actor shutdown against the sleep response itself.
	globalThis.setTimeout(() => {
		context.sleep();
	}, 0);
}

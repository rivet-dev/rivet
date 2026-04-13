export function scheduleActorSleep(context: { sleep: () => void }): void {
	context.sleep();
}

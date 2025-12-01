import confetti from "canvas-confetti";

export function successfulBackendSetupEffect() {
	confetti({
		angle: 60,
		spread: 55,
		origin: { x: 0 },
	});
	confetti({
		angle: 120,
		spread: 55,
		origin: { x: 1 },
	});
}

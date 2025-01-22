export function chooseRandomColor(): string {
	return `hsl(${Math.floor(Math.random() * 360)}, 60%, 60%)`;
}

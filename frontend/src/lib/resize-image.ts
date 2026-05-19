export async function resizeImageToDataUrl(
	file: File,
	maxSize = 256,
): Promise<string> {
	const bitmap = await createImageBitmap(file);
	const scale = Math.min(1, maxSize / Math.max(bitmap.width, bitmap.height));
	const width = Math.round(bitmap.width * scale);
	const height = Math.round(bitmap.height * scale);

	const canvas = document.createElement("canvas");
	canvas.width = width;
	canvas.height = height;
	const ctx = canvas.getContext("2d");
	if (!ctx) throw new Error("Canvas 2D context unavailable");
	ctx.drawImage(bitmap, 0, 0, width, height);
	bitmap.close();
	return canvas.toDataURL("image/jpeg", 0.85);
}

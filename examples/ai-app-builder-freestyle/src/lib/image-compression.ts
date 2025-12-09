export interface CompressedImage {
  data: string; // base64 encoded
  mimeType: string;
  originalSize: number;
  compressedSize: number;
}

export async function compressImage(
  file: File,
  maxWidth: number = 1024,
  maxHeight: number = 1024,
  quality: number = 0.8
): Promise<CompressedImage> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    const canvas = document.createElement("canvas");
    const ctx = canvas.getContext("2d");

    if (!ctx) {
      reject(new Error("Could not get canvas context"));
      return;
    }

    img.onload = () => {
      // Calculate new dimensions while maintaining aspect ratio
      let { width, height } = img;

      if (width > maxWidth || height > maxHeight) {
        const aspectRatio = width / height;

        if (width > height) {
          width = maxWidth;
          height = Math.round(maxWidth / aspectRatio);
        } else {
          height = maxHeight;
          width = Math.round(maxHeight * aspectRatio);
        }
      }

      // Set canvas dimensions
      canvas.width = width;
      canvas.height = height;

      // Draw and compress the image
      ctx.drawImage(img, 0, 0, width, height);

      // Determine output format and quality
      const outputFormat =
        file.type === "image/png" ? "image/png" : "image/jpeg";
      const compressedData = canvas.toDataURL(outputFormat, quality);

      // Calculate compressed size (rough estimation)
      const compressedSize = Math.round((compressedData.length * 3) / 4);

      resolve({
        data: compressedData,
        mimeType: outputFormat,
        originalSize: file.size,
        compressedSize,
      });
    };

    img.onerror = () => {
      // Fallback: just convert to base64 without compression
      const reader = new FileReader();
      reader.onload = () => {
        resolve({
          data: reader.result as string,
          mimeType: file.type,
          originalSize: file.size,
          compressedSize: file.size,
        });
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    };

    // Create object URL to load the image
    img.src = URL.createObjectURL(file);
  });
}

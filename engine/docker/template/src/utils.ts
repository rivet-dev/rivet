import * as fs from "node:fs";
import * as path from "node:path";

// Utility function to read fixture files
export function readFixture(fixturePath: string): string {
	const fullPath = path.join(__dirname, "../fixtures", fixturePath);
	return fs.readFileSync(fullPath, "utf8");
}

export function ensureDir(dirPath: string) {
	if (!fs.existsSync(dirPath)) {
		fs.mkdirSync(dirPath, { recursive: true });
	}
}

export function writeFile(filePath: string, content: string) {
	ensureDir(path.dirname(filePath));
	fs.writeFileSync(filePath, content);
}

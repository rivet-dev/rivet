const fs = require("fs");
const path = require("path");

// Remove the old generateLlmsTxt.ts file
const oldFilePath = path.join(__dirname, "scripts", "generateLlmsTxt.ts");

try {
	fs.unlinkSync(oldFilePath);
	console.log("Successfully removed generateLlmsTxt.ts");
} catch (error) {
	console.log("File may already be removed or not found:", error.message);
}

// Remove this cleanup script
fs.unlinkSync(__filename);

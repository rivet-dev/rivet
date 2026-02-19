/** @type {import('tailwindcss').Config} */
module.exports = {
	content: ["./src/**/*.{ts,tsx}", "./apps/**/*.{ts,tsx}"],
	presets: [require("./src/components/tailwind-base")],
};

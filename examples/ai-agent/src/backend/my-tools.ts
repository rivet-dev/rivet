export async function getWeather(location: string) {
	// Mock weather API response
	return {
		location,
		temperature: Math.floor(Math.random() * 30) + 10,
		condition: ["sunny", "cloudy", "rainy", "snowy"][
			Math.floor(Math.random() * 4)
		],
		humidity: Math.floor(Math.random() * 50) + 30,
	};
}

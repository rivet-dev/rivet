import { Outlet } from "react-router-dom";
import { Toaster } from "sonner";

function App() {
	return (
		<div className="min-h-screen bg-background text-foreground antialiased">
			<Toaster />
			<Outlet />
		</div>
	);
}

export default App;

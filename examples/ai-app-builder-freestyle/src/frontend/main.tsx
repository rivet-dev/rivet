import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route, Outlet } from "react-router-dom";
import { Toaster } from "sonner";
import HomePage from "./pages/HomePage";
import NewAppPage from "./pages/NewAppPage";
import AppEditorPage from "./pages/AppEditorPage";
import "./styles/globals.css";

function App() {
	return (
		<>
			<Outlet />
			<Toaster />
		</>
	);
}

ReactDOM.createRoot(document.getElementById("root")!).render(
	<React.StrictMode>
		<BrowserRouter>
			<Routes>
				<Route path="/" element={<App />}>
					<Route index element={<HomePage />} />
					<Route path="app/new" element={<NewAppPage />} />
					<Route path="app/:id" element={<AppEditorPage />} />
				</Route>
			</Routes>
		</BrowserRouter>
	</React.StrictMode>
);

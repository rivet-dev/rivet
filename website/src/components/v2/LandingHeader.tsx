"use client";

import { useEffect, useState } from "react";

export function LandingHeader() {
	const [isScrolled, setIsScrolled] = useState(false);

	useEffect(() => {
		const handleScroll = () => {
			setIsScrolled(window.scrollY > 50);
		};

		window.addEventListener("scroll", handleScroll);
		handleScroll();

		return () => window.removeEventListener("scroll", handleScroll);
	}, []);

	return (
		<header
			id="header"
			className="fixed top-0 left-0 right-0 z-50 transition-all duration-300"
			style={{
				backgroundColor: isScrolled ? 'rgba(10, 10, 10, 0.8)' : 'transparent',
				backdropFilter: isScrolled ? 'blur(16px)' : 'none',
				borderBottom: isScrolled ? '1px solid #252525' : 'none'
			}}
		>
			<div className="container mx-auto max-w-6xl px-4 sm:px-6 lg:px-8">
				<div className="flex h-20 items-center justify-between">
					<a href="/" className="flex items-center gap-2">
						<svg className="h-6 w-6" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" style={{ color: '#FF4500' }}>
							<path d="M4 4H20V12H4V4Z" fill="currentColor" />
							<path d="M4 14H10V20H4V14Z" fill="currentColor" />
							<path d="M14 14H20V20H14V14Z" fill="currentColor" />
						</svg>
						<span className="font-heading text-xl font-bold" style={{ color: '#FAFAFA' }}>Rivet</span>
					</a>
					<nav className="hidden md:flex items-center gap-6">
						<a href="/docs"
							className="text-sm font-medium transition-colors"
							style={{ color: '#A0A0A0' }}
						>
							Documentation
						</a>
						<a href="/blog"
							className="text-sm font-medium transition-colors"
							style={{ color: '#A0A0A0' }}
						>
							Blog
						</a>
						<a href="https://github.com/rivet-gg/rivet"
							className="text-sm font-medium transition-colors"
							style={{ color: '#A0A0A0' }}
						>
							GitHub →
						</a>
						<a href="/docs/quickstart/"
							className="px-4 py-2 rounded-lg text-sm font-medium text-white transition-all duration-200 hover:bg-orange-600"
							style={{ backgroundColor: '#FF4500' }}
						>
							Get Started
						</a>
					</nav>
					<div className="md:hidden">
						<button style={{ color: '#A0A0A0' }}>
							<svg
								className="h-6 w-6"
								xmlns="http://www.w3.org/2000/svg"
								fill="none"
								viewBox="0 0 24 24"
								strokeWidth="1.5"
								stroke="currentColor"
							>
								<path strokeLinecap="round" strokeLinejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" />
							</svg>
						</button>
					</div>
				</div>
			</div>
		</header>
	);
}

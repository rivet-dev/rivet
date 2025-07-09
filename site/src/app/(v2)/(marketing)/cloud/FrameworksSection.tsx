import Link from "next/link";
import {
	Icon,
	faReact,
	faVuejs,
	faAngular,
	faNodeJs,
	faPython,
	faPhp,
	faJava,
	faRust,
	faSwift,
	faJsSquare,
	faHtml5,
	faCss3Alt,
	faGolang,
	faDatabase,
	faDocker,
} from "@rivet-gg/icons";

// Frameworks section
export const FrameworksSection = () => {
	const frameworks = [
		{ icon: faReact, name: "React", href: "/docs/frameworks/react" },
		{ icon: faVuejs, name: "Vue", href: "/docs/frameworks/vue" },
		{ icon: faAngular, name: "Angular", href: "/docs/frameworks/angular" },
		{ icon: faNodeJs, name: "Node.js", href: "/docs/frameworks/nodejs" },
		{ icon: faPython, name: "Python", href: "/docs/frameworks/python" },
		{ icon: faPhp, name: "PHP", href: "/docs/frameworks/php" },
		{ icon: faJava, name: "Java", href: "/docs/frameworks/java" },
		{ icon: faRust, name: "Rust", href: "/docs/frameworks/rust" },
		{ icon: faSwift, name: "Swift", href: "/docs/frameworks/swift" },
		{
			icon: faJsSquare,
			name: "JavaScript",
			href: "/docs/frameworks/javascript",
		},
		{ icon: faHtml5, name: "HTML5", href: "/docs/frameworks/html5" },
		{ icon: faCss3Alt, name: "CSS3", href: "/docs/frameworks/css3" },
		{ icon: faGolang, name: "Go", href: "/docs/frameworks/go" },
		{ icon: faDatabase, name: "SQL", href: "/docs/frameworks/sql" },
		{ icon: faDocker, name: "Docker", href: "/docs/frameworks/docker" },
	];

	return (
		<div className="mx-auto max-w-7xl px-6 py-28 lg:py-44 lg:px-8 mt-16">
			<div className="flex flex-col md:flex-row md:items-start">
				<div className="grow max-w-lg mb-8 md:mb-0 md:pr-8">
					<h2 className="text-4xl font-medium tracking-tight text-white text-left">
						Rivet works with any framework
					</h2>
					<p className="mt-4 text-lg text-white/70 text-left">
						Integrate with your existing tech stack or start fresh
						with your preferred tools and languages.
					</p>
				</div>
				<div>
					<div className="grid grid-cols-4 gap-x-6 gap-y-6">
						{frameworks.map((framework, index) => (
							<Link
								key={index}
								href={framework.href}
								className="group"
							>
								<div className="h-16 w-16 mx-auto flex items-center justify-center">
									<Icon
										icon={framework.icon}
										className="text-5xl text-white/30 group-hover:text-white/90 transition-colors duration-200"
										title={framework.name}
									/>
								</div>
							</Link>
						))}
					</div>
				</div>
			</div>
		</div>
	);
};
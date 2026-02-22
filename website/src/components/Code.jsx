// "use client";
// import { Tab } from "@headlessui/react";
// import clsx from "clsx";
// import {
// 	Children,
// 	createContext,
// 	useContext,
// 	useEffect,
// 	useRef,
// 	useState,
// } from "react";
// import { create } from "zustand";
//
// import { Tag } from "@/components/Tag";
//
// const languageNames = {
// 	csharp: "C#",
// 	cpp: "C++",
// 	go: "Go",
// 	js: "JavaScript",
// 	json: "JSON",
// 	php: "PHP",
// 	python: "Python",
// 	ruby: "Ruby",
// 	ts: "TypeScript",
// 	yaml: "YAML",
// 	gdscript: "GDScript",
// 	docker: "Docker",
// 	rust: "Rust",
// };
//
// function getPanelTitle({ title, language }) {
// 	return title ?? languageNames[language] ?? "Code";
// }
//
// function ClipboardIcon(props) {
// 	return (
// 		<svg viewBox="0 0 20 20" aria-hidden="true" {...props}>
// 			<path
// 				strokeWidth="0"
// 				d="M5.5 13.5v-5a2 2 0 0 1 2-2l.447-.894A2 2 0 0 1 9.737 4.5h.527a2 2 0 0 1 1.789 1.106l.447.894a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-5a2 2 0 0 1-2-2Z"
// 			/>
// 			<path
// 				fill="none"
// 				strokeLinejoin="round"
// 				d="M12.5 6.5a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-5a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2m5 0-.447-.894a2 2 0 0 0-1.79-1.106h-.527a2 2 0 0 0-1.789 1.106L7.5 6.5m5 0-1 1h-3l-1-1"
// 			/>
// 		</svg>
// 	);
// }
//
// function CopyButton({ code }) {
// 	const [copyCount, setCopyCount] = useState(0);
// 	const copied = copyCount > 0;
//
// 	useEffect(() => {
// 		if (copyCount > 0) {
// 			const timeout = setTimeout(() => setCopyCount(0), 1000);
// 			return () => {
// 				clearTimeout(timeout);
// 			};
// 		}
// 	}, [copyCount]);
//
// 	return (
// 		<button
// 			type="button"
// 			className={clsx(
// 				"group/button absolute right-4 top-3.5 overflow-hidden rounded-full py-1 pl-2 pr-3 text-2xs font-medium opacity-0 backdrop-blur transition focus:opacity-100 group-hover:opacity-100",
// 				copied
// 					? "bg-violet-400/10 ring-1 ring-inset ring-violet-400/20"
// 					: "bg-white/5 hover:bg-white/7.5 dark:bg-white/2.5 dark:hover:bg-white/5",
// 			)}
// 			onClick={() => {
// 				window.navigator.clipboard.writeText(code).then(() => {
// 					setCopyCount((count) => count + 1);
// 				});
// 			}}
// 		>
// 			<span
// 				aria-hidden={copied}
// 				className={clsx(
// 					"pointer-events-none flex items-center gap-0.5 text-cream-400 transition duration-300",
// 					copied && "-translate-y-1.5 opacity-0",
// 				)}
// 			>
// 				<ClipboardIcon className="h-5 w-5 fill-charcole-500/20 stroke-charcole-500 transition-colors group-hover/button:stroke-cream-400" />
// 				Copy
// 			</span>
// 			<span
// 				aria-hidden={!copied}
// 				className={clsx(
// 					"pointer-events-none absolute inset-0 flex items-center justify-center text-violet-400 transition duration-300",
// 					!copied && "translate-y-1.5 opacity-0",
// 				)}
// 			>
// 				Copied!
// 			</span>
// 		</button>
// 	);
// }
//
// function CodePanelHeader({ tag, label }) {
// 	if (!tag && !label) {
// 		return null;
// 	}
//
// 	return (
// 		<div className="flex h-9 items-center gap-2 border-y border-b-white/7.5 border-t-transparent bg-neutral-950 px-4 dark:border-b-white/5">
// 			{tag && (
// 				<div className="dark flex">
// 					<Tag variant="small">{tag}</Tag>
// 				</div>
// 			)}
// 			{tag && label && (
// 				<span className="h-0.5 w-0.5 rounded-full bg-cream-500" />
// 			)}
// 			{label && (
// 				<span className="font-mono text-xs text-cream-400">
// 					{label}
// 				</span>
// 			)}
// 		</div>
// 	);
// }
//
// function CodePanel({ tag, label, code, children }) {
// 	if (!children) {
// 		return null;
// 	}
// 	const child = Children.only(children);
//
// 	return (
// 		<div className="group bg-neutral-950">
// 			<CodePanelHeader
// 				tag={child.props.tag ?? tag}
// 				label={child.props.label ?? label}
// 			/>
// 			<div className="relative">
// 				<pre className="overflow-x-auto p-4 text-xs text-white">
// 					{children}
// 				</pre>
// 				<CopyButton code={child.props.code ?? code} />
// 			</div>
// 		</div>
// 	);
// }
//
// function CodeGroupHeader({ title, children, selectedIndex }) {
// 	const hasTabs = Children.count(children) > 1;
//
// 	if (!title && !hasTabs) {
// 		return null;
// 	}
//
// 	return (
// 		<div className="flex min-h-[calc(theme(spacing.12)+1px)] flex-wrap items-start gap-x-4 border-b border-neutral-800 bg-neutral-950 px-4">
// 			{title && (
// 				<h3 className="mr-auto pt-3 font-sans text-xs font-semibold text-white">
// 					{title}
// 				</h3>
// 			)}
// 			{hasTabs && (
// 				<Tab.List className="-mb-px flex gap-4 text-xs font-medium">
// 					{Children.map(children, (child, childIndex) => {
// 						return (
// 							<Tab
// 								className={clsx(
// 									"border-b py-3 transition focus:[&:not(:focus-visible)]:outline-none",
// 									childIndex === selectedIndex
// 										? "border-violet-500 text-violet-400"
// 										: "border-transparent text-cream-400 hover:text-cream-100",
// 								)}
// 							>
// 								{getPanelTitle(child.props)}
// 							</Tab>
// 						);
// 					})}
// 				</Tab.List>
// 			)}
// 		</div>
// 	);
// }
//
// function CodeGroupPanels({ children, ...props }) {
// 	const hasTabs = Children.count(children) > 1;
//
// 	if (hasTabs) {
// 		return (
// 			<Tab.Panels>
// 				{Children.map(children, (child) => (
// 					<Tab.Panel>
// 						<CodePanel {...props}>{child}</CodePanel>
// 					</Tab.Panel>
// 				))}
// 			</Tab.Panels>
// 		);
// 	}
//
// 	return <CodePanel {...props}>{children}</CodePanel>;
// }
//
// function usePreventLayoutShift() {
// 	const positionRef = useRef();
// 	const rafRef = useRef();
//
// 	useEffect(() => {
// 		return () => {
// 			window.cancelAnimationFrame(rafRef.current);
// 		};
// 	}, []);
//
// 	return {
// 		positionRef,
// 		preventLayoutShift(callback) {
// 			const initialTop = positionRef.current.getBoundingClientRect().top;
//
// 			callback();
//
// 			rafRef.current = window.requestAnimationFrame(() => {
// 				const newTop = positionRef.current.getBoundingClientRect().top;
// 				window.scrollBy(0, newTop - initialTop);
// 			});
// 		},
// 	};
// }
//
// const usePreferredLanguageStore = create((set) => ({
// 	preferredLanguages: [],
// 	addPreferredLanguage: (language) =>
// 		set((state) => ({
// 			preferredLanguages: [
// 				...state.preferredLanguages.filter(
// 					(preferredLanguage) => preferredLanguage !== language,
// 				),
// 				language,
// 			],
// 		})),
// }));
//
// function useTabGroupProps(availableLanguages) {
// 	const { preferredLanguages, addPreferredLanguage } =
// 		usePreferredLanguageStore();
// 	const [selectedIndex, setSelectedIndex] = useState(0);
// 	const activeLanguage = [...availableLanguages].sort(
// 		(a, z) => preferredLanguages.indexOf(z) - preferredLanguages.indexOf(a),
// 	)[0];
// 	const languageIndex = availableLanguages.indexOf(activeLanguage);
// 	const newSelectedIndex =
// 		languageIndex === -1 ? selectedIndex : languageIndex;
// 	if (newSelectedIndex !== selectedIndex) {
// 		setSelectedIndex(newSelectedIndex);
// 	}
//
// 	const { positionRef, preventLayoutShift } = usePreventLayoutShift();
//
// 	return {
// 		as: "div",
// 		ref: positionRef,
// 		selectedIndex,
// 		onChange: (newSelectedIndex) => {
// 			preventLayoutShift(() =>
// 				addPreferredLanguage(availableLanguages[newSelectedIndex]),
// 			);
// 		},
// 	};
// }
//
// const CodeGroupContext = createContext(false);
//
// export function CodeGroup({ children, title, ...props }) {
// 	const languages = Children.map(children, (child) =>
// 		getPanelTitle(child.props),
// 	);
// 	const tabGroupProps = useTabGroupProps(languages);
// 	const hasTabs = Children.count(children) > 1;
// 	const Container = hasTabs ? Tab.Group : "div";
// 	const containerProps = hasTabs ? tabGroupProps : {};
// 	const headerProps = hasTabs
// 		? { selectedIndex: tabGroupProps.selectedIndex }
// 		: {};
//
// 	return (
// 		<CodeGroupContext.Provider value={true}>
// 			<Container
// 				{...containerProps}
// 				className="not-prose my-6 overflow-hidden rounded-2xl bg-neutral-950 shadow-md dark:ring-1 dark:ring-white/10"
// 			>
// 				<CodeGroupHeader title={title} {...headerProps}>
// 					{children}
// 				</CodeGroupHeader>
// 				<CodeGroupPanels {...props}>{children}</CodeGroupPanels>
// 			</Container>
// 		</CodeGroupContext.Provider>
// 	);
// }
//
// export function EphermeralTab({ children, ...props }) {
// 	return <>{children}</>;
// }
//
// export function Code({ children, ...props }) {
// 	const isGrouped = useContext(CodeGroupContext);
//
// 	if (isGrouped) {
// 		return (
// 			<code {...props} dangerouslySetInnerHTML={{ __html: children }} />
// 		);
// 	}
//
// 	return (
// 		<code className="whitespace-break-spaces break-words" {...props}>
// 			{children}
// 		</code>
// 	);
// }
//
// export function Pre({ children, ...props }) {
// 	const isGrouped = useContext(CodeGroupContext);
//
// 	if (isGrouped) {
// 		return children;
// 	}
//
// 	return <CodeGroup {...props}>{children}</CodeGroup>;
// }

/// <reference types="vite/client" />

interface ImportMetaEnv {
	readonly VITE_RIVET_PUBLIC_ENDPOINT?: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}

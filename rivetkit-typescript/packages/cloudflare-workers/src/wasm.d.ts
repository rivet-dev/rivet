declare module "*.wasm" {
	const wasmModule: WebAssembly.Module;
	export default wasmModule;
}

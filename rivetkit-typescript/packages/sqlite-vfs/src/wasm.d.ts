/**
 * Minimal WebAssembly type declarations for Node.js environments where
 * lib.dom.d.ts is not included. Only the types needed by the VFS pool
 * module caching are declared here.
 */
declare namespace WebAssembly {
	class Module {
		constructor(bytes: BufferSource);
	}

	class Instance {
		readonly exports: Exports;
		constructor(module: Module, importObject?: Imports);
	}

	type Imports = Record<string, Record<string, ImportValue>>;
	type ImportValue = Function | Global | Memory | Table | number;
	type Exports = Record<string, Function | Global | Memory | Table | number>;

	class Global {
		constructor(descriptor: GlobalDescriptor, value?: number);
		value: number;
	}

	interface GlobalDescriptor {
		value: string;
		mutable?: boolean;
	}

	class Memory {
		constructor(descriptor: MemoryDescriptor);
		readonly buffer: ArrayBuffer;
	}

	interface MemoryDescriptor {
		initial: number;
		maximum?: number;
	}

	class Table {
		constructor(descriptor: TableDescriptor);
		readonly length: number;
	}

	interface TableDescriptor {
		element: string;
		initial: number;
		maximum?: number;
	}

	function compile(bytes: BufferSource): Promise<Module>;
	function instantiate(
		module: Module,
		importObject?: Imports,
	): Promise<Instance>;
	function instantiate(
		bytes: BufferSource,
		importObject?: Imports,
	): Promise<{ module: Module; instance: Instance }>;
}

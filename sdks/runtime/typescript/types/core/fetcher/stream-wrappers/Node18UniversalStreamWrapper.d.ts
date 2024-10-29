/// <reference types="node" />
import type { Writable } from "stream";
import { EventCallback, StreamWrapper } from "./chooseStreamWrapper";
export declare class Node18UniversalStreamWrapper implements StreamWrapper<Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array>, Uint8Array> {
    private readableStream;
    private reader;
    private events;
    private paused;
    private resumeCallback;
    private encoding;
    constructor(readableStream: ReadableStream<Uint8Array>);
    on(event: string, callback: EventCallback): void;
    off(event: string, callback: EventCallback): void;
    pipe(dest: Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array>): Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array>;
    pipeTo(dest: Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array>): Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array>;
    unpipe(dest: Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array>): void;
    destroy(error?: Error): void;
    pause(): void;
    resume(): void;
    get isPaused(): boolean;
    read(): Promise<Uint8Array | undefined>;
    setEncoding(encoding: string): void;
    text(): Promise<string>;
    json<T>(): Promise<T>;
    private _write;
    private _end;
    private _error;
    private _emit;
    private _startReading;
}

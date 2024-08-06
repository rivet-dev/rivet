import type { Writable } from "stream";
import { EventCallback, StreamWrapper } from "./chooseStreamWrapper";

export class Node18UniversalStreamWrapper
    implements StreamWrapper<Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array>, Uint8Array>
{
    private readableStream: ReadableStream<Uint8Array>;
    private reader: ReadableStreamDefaultReader<Uint8Array>;
    private events: Record<string, EventCallback[] | undefined>;
    private paused: boolean;
    private resumeCallback: ((value?: unknown) => void) | null;
    private encoding: string | null;

    constructor(readableStream: ReadableStream<Uint8Array>) {
        this.readableStream = readableStream;
        this.reader = this.readableStream.getReader();
        this.events = {
            data: [],
            end: [],
            error: [],
            readable: [],
            close: [],
            pause: [],
            resume: [],
        };
        this.paused = false;
        this.resumeCallback = null;
        this.encoding = null;
    }

    public on(event: string, callback: EventCallback): void {
        this.events[event]?.push(callback);
    }

    public off(event: string, callback: EventCallback): void {
        this.events[event] = this.events[event]?.filter((cb) => cb !== callback);
    }

    public pipe(
        dest: Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array>
    ): Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array> {
        this.on("data", async (chunk) => {
            if (dest instanceof Node18UniversalStreamWrapper) {
                dest._write(chunk);
            } else if (dest instanceof WritableStream) {
                const writer = dest.getWriter();
                writer.write(chunk).then(() => writer.releaseLock());
            } else {
                dest.write(chunk);
            }
        });

        this.on("end", async () => {
            if (dest instanceof Node18UniversalStreamWrapper) {
                dest._end();
            } else if (dest instanceof WritableStream) {
                const writer = dest.getWriter();
                writer.close();
            } else {
                dest.end();
            }
        });

        this.on("error", async (error) => {
            if (dest instanceof Node18UniversalStreamWrapper) {
                dest._error(error);
            } else if (dest instanceof WritableStream) {
                const writer = dest.getWriter();
                writer.abort(error);
            } else {
                dest.destroy(error);
            }
        });

        this._startReading();

        return dest;
    }

    public pipeTo(
        dest: Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array>
    ): Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array> {
        return this.pipe(dest);
    }

    public unpipe(dest: Node18UniversalStreamWrapper | Writable | WritableStream<Uint8Array>): void {
        this.off("data", async (chunk) => {
            if (dest instanceof Node18UniversalStreamWrapper) {
                dest._write(chunk);
            } else if (dest instanceof WritableStream) {
                const writer = dest.getWriter();
                writer.write(chunk).then(() => writer.releaseLock());
            } else {
                dest.write(chunk);
            }
        });

        this.off("end", async () => {
            if (dest instanceof Node18UniversalStreamWrapper) {
                dest._end();
            } else if (dest instanceof WritableStream) {
                const writer = dest.getWriter();
                writer.close();
            } else {
                dest.end();
            }
        });

        this.off("error", async (error) => {
            if (dest instanceof Node18UniversalStreamWrapper) {
                dest._error(error);
            } else if (dest instanceof WritableStream) {
                const writer = dest.getWriter();
                writer.abort(error);
            } else {
                dest.destroy(error);
            }
        });
    }

    public destroy(error?: Error): void {
        this.reader
            .cancel(error)
            .then(() => {
                this._emit("close");
            })
            .catch((err) => {
                this._emit("error", err);
            });
    }

    public pause(): void {
        this.paused = true;
        this._emit("pause");
    }

    public resume(): void {
        if (this.paused) {
            this.paused = false;
            this._emit("resume");
            if (this.resumeCallback) {
                this.resumeCallback();
                this.resumeCallback = null;
            }
        }
    }

    public get isPaused(): boolean {
        return this.paused;
    }

    public async read(): Promise<Uint8Array | undefined> {
        if (this.paused) {
            await new Promise((resolve) => {
                this.resumeCallback = resolve;
            });
        }
        const { done, value } = await this.reader.read();

        if (done) {
            return undefined;
        }
        return value;
    }

    public setEncoding(encoding: string): void {
        this.encoding = encoding;
    }

    public async text(): Promise<string> {
        const chunks: Uint8Array[] = [];

        while (true) {
            const { done, value } = await this.reader.read();
            if (done) break;
            if (value) chunks.push(value);
        }

        const decoder = new TextDecoder(this.encoding || "utf-8");
        return decoder.decode(await new Blob(chunks).arrayBuffer());
    }

    public async json<T>(): Promise<T> {
        const text = await this.text();
        return JSON.parse(text);
    }

    private _write(chunk: Uint8Array): void {
        this._emit("data", chunk);
    }

    private _end(): void {
        this._emit("end");
    }

    private _error(error: any): void {
        this._emit("error", error);
    }

    private _emit(event: string, data?: any): void {
        if (this.events[event]) {
            for (const callback of this.events[event] || []) {
                callback(data);
            }
        }
    }

    private async _startReading(): Promise<void> {
        try {
            this._emit("readable");
            while (true) {
                if (this.paused) {
                    await new Promise((resolve) => {
                        this.resumeCallback = resolve;
                    });
                }
                const { done, value } = await this.reader.read();
                if (done) {
                    this._emit("end");
                    this._emit("close");
                    break;
                }
                if (value) {
                    this._emit("data", value);
                }
            }
        } catch (error) {
            this._emit("error", error);
        }
    }
}

import { Actor, Rpc } from "@rivet-gg/actor";

interface State {
    messages: string[];
}

export default class Chat extends Actor<State> {
    override _onInitialize() {
        return {
            messages: [],
        };
    }

    sendMessage(_rpc: Rpc<this>, message: string) {
        this._state.messages.push(message);
        this._state.messages = this._state.messages.slice(-10);
        this._broadcast("newMessage", { messages: this._state.messages });
        return this._state.messages;
    }

    getMessages() {
        return this._state.messages || [];
    }
}

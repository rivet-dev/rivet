export default function StepDefineActor() {
  return (
    <div className="mb-4">
      <p>Define your actor schema and implementation. Create a new file for your actor:</p>
      
      <div className="mt-3 p-4 bg-gray-50 rounded-lg border">
        <pre className="text-sm"><code>{`// actors/counter.ts
import { Actor } from '@rivetkit/actor';

export interface CounterState {
  count: number;
}

export class CounterActor extends Actor<CounterState> {
  getInitialState() {
    return { count: 0 };
  }

  increment() {
    this.setState({ count: this.state.count + 1 });
  }

  decrement() {
    this.setState({ count: this.state.count - 1 });
  }
}`}</code></pre>
      </div>
    </div>
  );
}
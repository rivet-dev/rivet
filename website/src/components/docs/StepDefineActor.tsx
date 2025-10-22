import { CodeBlock } from '../CodeBlock';
import { Code } from '../v2/Code';

export default function StepDefineActor() {
  return (
    <div className='mb-4'>
      <p>Define your actor schema and implementation. Create a new file for your actor:</p>

      <Code language='typescript' title='actors/counter.ts'>
        <CodeBlock
          code={`import { Actor } from 'rivetkit';

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
}`}
          lang='typescript'
        />
      </Code>
    </div>
  );
}

'use client';
import { Slot, toast } from '@rivet-gg/components';

export function CopyCodeTrigger({ children }) {
  const handleClick = event => {
    const code = event.currentTarget.parentNode.parentNode.querySelector('.code').innerText;
    navigator.clipboard.writeText(code);
    toast.success('Copied to clipboard');
  };
  return <Slot onClick={handleClick}>{children}</Slot>;
}

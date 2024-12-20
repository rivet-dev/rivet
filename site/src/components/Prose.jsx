import clsx from 'clsx';

export function Prose({ as: Component = 'div', className = '', ...props }) {
  return <Component className={clsx(className, 'prose-invert prose')} {...props} />;
}

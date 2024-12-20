import clsx from "clsx";

export function VintageBar({ className, ...props }) {
    let colors = ['bg-cream-100', 'bg-cream-200'];
    return (
        <div className={clsx(className, 'flex')} {...props}>
            {colors.map(c => (<div key={c} className={clsx(c, 'w-full h-full')}></div>))}
        </div>
    )
}
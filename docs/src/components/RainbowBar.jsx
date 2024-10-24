import clsx from "clsx";

export function RainbowBar({ className, ...props }) {
    let colors = ['bg-red-500', 'bg-orange-500', 'bg-yellow-500', 'bg-green-500', 'bg-blue-500', 'bg-purple-500'];
    return (
        <div className={clsx(className, 'flex')} {...props}>
            {colors.map(c => (<div key={c} className={clsx(c, 'w-full h-full')}></div>))}
        </div>
    )
}
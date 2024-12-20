import clsx from "clsx";

export function RainbowBarAnimated({ className, ...props }) {
    let colorClasses = {
        'bg-red-500': 'red-glow',
        'bg-orange-500': 'orange-glow',
        'bg-yellow-500': 'yellow-glow',
        'bg-green-500': 'green-glow',
        'bg-blue-500': 'blue-glow',
        'bg-purple-500': 'purple-glow'
    };

    return (
        <div className={clsx(className, 'flex')} {...props}>
            {Object.keys(colorClasses).map(color => (
                <div 
                    key={color}
                    className={clsx(color, colorClasses[color], 'w-full h-full')}
                ></div>
            ))}
        </div>
    );
}

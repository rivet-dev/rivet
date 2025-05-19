"use client";

import { useState } from 'react';
import { Icon } from "@rivet-gg/icons";
import { faCheck, faCopy } from "@fortawesome/free-solid-svg-icons";

interface CopyCommandProps {
    command: string;
}

export const CopyCommand: React.FC<CopyCommandProps> = ({ command }) => {
    const [copied, setCopied] = useState(false);
    const [isPressed, setIsPressed] = useState(false);

    const handleCopy = async () => {
        await navigator.clipboard.writeText(command);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };

    return (
        <button 
            onClick={handleCopy}
            onMouseDown={() => setIsPressed(true)}
            onMouseUp={() => setIsPressed(false)}
            onMouseLeave={() => setIsPressed(false)}
            className={`relative group inline-flex items-center bg-white/5 rounded-lg border border-white/10 hover:border-white/20 transition-all duration-75 ${
                isPressed ? 'transform scale-[0.98] bg-white/[0.03]' : ''
            }`}
            aria-label="Copy command"
        >
            <code className="px-4 py-2 text-sm text-white/70 font-mono cursor-pointer">
                {command}
            </code>
            <div className="px-3 py-2 border-l border-white/10 text-white/40 group-hover:text-white/90 transition-colors">
                <Icon
                    icon={copied ? faCheck : faCopy}
                    className={`text-sm ${copied ? 'text-green-500' : ''}`}
                />
            </div>
        </button>
    );
};
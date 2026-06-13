import type { ReactNode } from 'react';
import { CAPTION_CLASS } from '../typography';

// Classical image in a museum mat: warm cream mat, hairline frame, and a
// printed catalog caption below, e.g.
// "Fig. 01 — A Lady Writing · Johannes Vermeer · c. 1665". Hook-free.
interface PlateImageProps {
	src: string;
	alt: string;
	figure?: string;
	title?: string;
	artist?: string;
	date?: string;
	width?: number;
	height?: number;
	loading?: 'eager' | 'lazy';
	children?: ReactNode;
	className?: string;
}

export const PlateImage = ({
	src,
	alt,
	figure,
	title,
	artist,
	date,
	width,
	height,
	loading = 'lazy',
	children,
	className,
}: PlateImageProps) => {
	const captionParts = [title, artist, date].filter(Boolean);
	return (
		<figure className={className}>
			<div className="border border-ink/15 bg-paper-mid p-3">
				{children ?? (
					<img
						src={src}
						alt={alt}
						width={width}
						height={height}
						loading={loading}
						className="block h-auto w-full outline outline-1 outline-ink/10"
					/>
				)}
			</div>
			{captionParts.length > 0 ? (
				<figcaption className={`${CAPTION_CLASS} mt-4`}>
					{figure ? <span className="font-medium text-ink-soft">{figure} — </span> : null}
					{captionParts.join(' · ')}
				</figcaption>
			) : null}
		</figure>
	);
};

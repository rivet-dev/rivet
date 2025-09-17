export function TutorialVideo({ videoId }) {
	return (
		<div className="relative w-full" style={{ paddingBottom: '56.25%' }}>
			<iframe
				className="absolute top-0 left-0 w-full h-full"
				src={`https://www.youtube-nocookie.com/embed/${videoId}`}
				title="YouTube video player"
				frameBorder="0"
				allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
				allowFullScreen
			/>
		</div>
	);
}

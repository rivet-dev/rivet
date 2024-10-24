export function TutorialVideo({ videoId }) {
  return (
    <iframe
      width='560'
      height='315'
      src={`https://www.youtube-nocookie.com/embed/${videoId}`}
      title='YouTube video player'
      frameBorder='0'
      allow='accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share'
      allowFullScreen
    />
  );
}

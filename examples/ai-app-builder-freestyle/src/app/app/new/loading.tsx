import "@/components/loader.css";

export default function Loading() {
  return (
    <div className="flex justify-center items-center h-screen">
      <div>
        <div className="text-center">Creating App</div>
        <div className="loader"></div>
      </div>
    </div>
  );
}

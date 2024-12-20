import { Header } from '@/components/v2/Header';

export default function Layout({ children }) {
  return (
    <>
      <Header active='blog' />
      <div className='mx-auto w-full max-w-6xl px-8'>{children}</div>
    </>
  );
}

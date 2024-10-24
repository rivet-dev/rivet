import clsx from 'clsx';
import { forwardRef, useState, useEffect, Fragment } from 'react';
import { usePostHog } from 'posthog-js/react';
import { useRouter } from 'next/router';
import { Transition } from '@headlessui/react';
import { CheckIcon } from '@heroicons/react/20/solid';
import { Button } from '@/components/Button';
import { faGithub } from '@rivet-gg/icons';
import { RainbowBar } from '@/components/RainbowBar';

export function Feedback() {
  const posthog = usePostHog();

  let router = useRouter();
  let feedbackKey = `feedback:${router.pathname}`;
  let [submitted, setSubmitted] = useState(false);

  // Populate submitted
  useEffect(() => {
    if (localStorage.getItem(feedbackKey)) {
      setSubmitted(true);
    }
  }, [feedbackKey]);

  // Handle submission
  function onSubmit(event) {
    event.preventDefault();

    // Send event
    posthog?.capture('page_feedback', {
      page: router.pathname,
      helpful: event.nativeEvent.submitter.dataset.response === 'yes'
    });

    // Update state
    localStorage.setItem(feedbackKey, 'true');
    setSubmitted(true);
  }

  return (
    <div>
      <RainbowBar className={clsx('mb-4 h-1', 'mx-auto max-w-5xl')} />
      <div className='relative h-8'>
        <Transition
          show={!submitted}
          as={Fragment}
          leaveFrom='opacity-100'
          leaveTo='opacity-0'
          leave='pointer-events-none duration-300'>
          <FeedbackForm onSubmit={onSubmit} />
        </Transition>
        <Transition
          show={submitted}
          as={Fragment}
          enterFrom='opacity-0'
          enterTo='opacity-100'
          enter='delay-150 duration-300'>
          <FeedbackThanks />
        </Transition>
      </div>
    </div>
  );
}

function EditButton() {
  let router = useRouter();
  let href = `https://github.com/rivet-gg/site/edit/main/src/pages${router.pathname}.mdx`;

  return (
    <Button variant='text-subtle' icon={faGithub} href={href} target='_blank'>
      Edit Page
    </Button>
  );
}

const FeedbackForm = forwardRef(function FeedbackForm({ onSubmit }, ref) {
  return (
    <form
      ref={ref}
      onSubmit={onSubmit}
      className={clsx(
        'absolute inset-0 flex items-center justify-center gap-4 md:justify-start',
        'mx-auto max-w-5xl'
      )}>
      {/* Left */}
      <p className='text-sm text-charcole-600 dark:text-cream-100'>Was this page helpful?</p>
      <div className='flex gap-2 font-bold'>
        <FeedbackButton data-response='yes'>Yes</FeedbackButton>
        <FeedbackButton data-response='no'>No</FeedbackButton>
      </div>

      {/* Spacer */}
      <div className='flex-1' />

      {/* Right */}
      <EditButton />
    </form>
  );
});

function FeedbackButton(props) {
  return (
    <Button
      type='submit'
      variant='text-subtle'
      // className='px-3 text-sm font-medium text-charcole-600 transition dark:text-cream-100 hover:bg-white/5 hover:text-white'
      className='font-bold'
      {...props}
    />
  );
}

const FeedbackThanks = forwardRef(function FeedbackThanks(_props, ref) {
  return (
    <div
      ref={ref}
      className={clsx(
        'absolute inset-0 flex items-center justify-center gap-4 md:justify-start',
        'mx-auto max-w-5xl'
      )}>
      {/* Left */}
      <div className='mr-4 flex items-center bg-orange-50/50 py-1 pl-2 pr-3 text-sm text-orange-900 ring-1 ring-inset ring-orange-500/20 dark:bg-orange-500/5 dark:text-orange-200 dark:ring-orange-500/30'>
        <CheckIcon className='mr-3 h-5 w-5 flex-none fill-white' />
        Thanks for your feedback!
      </div>

      {/* Spacer */}
      <div className='flex-1' />

      {/* Right */}
      <EditButton />
    </div>
  );
});

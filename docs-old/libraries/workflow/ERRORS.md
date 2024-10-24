# Errors

## Retries

Only errors from inside of activities will be retried. Errors thrown in the workflow body will not be retried
because they will never succeed (the state is consistent up the point of error).

## Workflow errors

Sub workflow errors cannot be caught because it's up to the workflow to handle its own errors gracefully.

We return OK responses from workflows for failure cases that we will explicitly handle (e.g. linode server
provision cleaning itself up). See
[Errors that are meant to be propagated up](#errors-that-are-meant-to-be-propagated-up).

## Propagation

There are 3 classes of errors in workflows:

1. Errors that can't be retried
2. Errors that can be retried
3. Errors that are meant to be propagated up

### Errors that can't be retried

Certain errors cannot be retried by the workflow system. These are usually problems with the internal
mechanisms of the workflow system itself.

### Errors that can be retried

All user errors thrown in an activity will cause a workflow retry. While this is good for errors meant to be
retried, it causes unnecessary retries for errors that you know can't be recovered from (like assertions). We
don't currently have a way to mitigate this besides propagating the errors manually (see below) or letting the
useless retries happen.

### Errors that are meant to be propagated up

To propagate an error, you must manually serialize it in the activity/workflow output. The workflow itself
will succeed, but the output data will have the error you want to propagate up.

You can use nested `Result`'s for this:

```rust
#[derive(...)]
struct MyActivityInput { }

type MyActivityOutput = Result<MyActivityOutputOk, MyActivityOutputErr>;

#[derive(...)]
struct MyActivityOutputOk {
	foo: String,
}

#[derive(...)]
struct MyActivityOutputErr {
	bar: u32,
}

fn activity(input: MyActivityInput) -> GlobalResult<MyActivityOutput> {
	if ... {
		return Ok(Err(MyActivityOutputErr {
			bar: 404,
		}));
	}

	Ok(Ok(MyActivityOutputOk {
		foo: "all good".to_string(),
	}))
}
```

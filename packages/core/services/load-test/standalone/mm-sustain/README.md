# mm-sustain

This load test is meant to catch sporadic errors in the matchmaker that may not show up during routine tests.
This works by spawning X workers in parallel that repeatedly create, connect to, then destroys a lobby. It can
be updated to pause on a lobby that has a problem detected so it can be diagnosed manually.

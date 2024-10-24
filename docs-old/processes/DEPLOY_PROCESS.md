# Deploy process

When merging code:

1. Pull request gets merged
2. Code gets deployed to `staging` & issues convert to _Validating_

When publishing to production:

1. All _Validating_ issues are manually validated on staging
2. Publish deploy
3. Check all _Validating_ issues again and set to _Complete_
   - If there is a problem, complete the issue and create a new issue

# Debugging

## Testing release workflow

It's common to need to test or iterate on the release workflow.

Set `branch` to the current branch and `version` to `x.x.x-rc.x` for a release candidate.

Make sure to disable the automatic merging of the Release Please commit.

```
gt m -a && gt s --force && gh workflow run .github/workflows/release.yaml --ref BRANCH -f version=VERSION -f latest=false
```

`--force` is because this workflow will push commits automatically.


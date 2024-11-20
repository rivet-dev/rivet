# Git Dependencies

We opt to use submodules instead of Git dependencies because of complications with building:

- Primariy, there are bugs with the Cargo git fetcher that cause it to not be
  able to pull from GitHub Actions
- Development on externals is easier when developing in-place with a Git
  submodule instead of having to push & update the ref every iteration
- Authenticating Git inside of a Docker build step is dangerous process that
  can lead to accidentally leaking credentials
	- Currently, we support mounting a netrc secret which is safe, but this
	  doesn't seem to solve the bugs with the Cargo git fetcher


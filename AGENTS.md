## Changes
- Changes for each feature should be staged for commit but shouldn't be commited automatically if there is no explicit command from user

## Build instructions
- For building the application in development use `cargo build`
- For building the application for production readiness use `./scripts/build-macos.sh`
- All finalized features should be built for production readiness before finishing

## Code Style
- All errors shouldn't be left without attention and should be logged as `error!` traces
- Run `cargo fmt` before every commit

## Debug
- Lookup application debug logs in user home directory in folder `.push-to-talk/logs/`

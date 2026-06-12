## Changes
- Changes for each feature should be staged for commit but shouldn't be committed automatically if there is no explicit command from user

## Software design decisions
- All solutions created should be good for installing on MacOS and Windows

## Build instructions
- Frontend: `cd ui && npm run build` (production) or `npm run dev` (development)
- For development with hot-reload: `cargo tauri dev`
- For release: `cargo tauri build` (bundles frontend + backend into installer)
- macOS packaging: `./scripts/build-macos.sh`

## Frontend stack
- React 18 + TypeScript + Vite 5
- Bootstrap 5 (Morph dark theme via Bootswatch)
- i18n: 49 languages (EN, RU, DE, FR, ES, IT, PT, PL, UA, NL, CS, SV, FI, RO, HU, EL, BG, DA, SK, LT, LV, ET, SL, HR, NO, TR, BE, ZH, JA, KO, HI, AR, TH, VI, ID, MS, FA, HE, BN, UR, TA, TE, SW, AM, ZU, AF, HA, YO, PT-BR)
- Tauri IPC via `.bind()`-fixed `window.__TAURI__.core.invoke` in `ui/src/services/tauri.ts`

## Code Style
- All errors shouldn't be left without attention and should be logged as `error!` traces
- Run `cargo fmt` before every commit

## Debug
- Lookup application debug logs in user home directory in folder `.push-to-talk/logs/`

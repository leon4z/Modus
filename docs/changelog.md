# Modus Changelog

- Status: active
- Source-of-Truth: false
- Type: note

## 1.0.5 - 2026-06-07

- Added separate App and CLI presence evidence for adapted coding tools.
- Showed tool presence as independent APP and CLI badges in Settings instead of a combined state.
- Updated tool research and onboarding notes to record official app/CLI detection evidence before future adapters are integrated.

## 1.0.4 - 2026-06-04

- Updated managed Global Rule block markers to use the Modus product name while preserving compatibility with existing managed blocks.
- Kept legacy managed blocks in sync when their rule content matches, and upgraded them to the new marker names on the next explicit injection.
- Prevented malformed managed block markers from being overwritten or duplicated during sync.

## 1.0.3 - 2026-06-03

- Made the Settings GitHub action open the public Modus project page.

## 1.0.2 - 2026-06-03

- Removed the app icon's outer rim highlight so Finder, Dock, and app preview surfaces show a cleaner icon edge.

## 1.0.1 - 2026-06-03

- Improved CLI tool detection for installed macOS builds by checking common local binary directories when the app is launched without a shell PATH.

## 1.0.0 - 2026-06-03

- Prepared the Modus mainline around the retained local core workflows: Dashboard, Rules, Skills, Config, MCP, and Settings.
- Removed public exposure for advanced workflows that are not part of the current product surface.
- Unified the public product name, README, window title, app chrome, and Settings/About surface around Modus.
- Added Modus app update checks with startup detection, a Settings navigation update tag, explicit install/restart actions, and separate stable/test release manifests.

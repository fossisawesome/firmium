# Agents Guidelines

Behavioral guidelines for autonomous and semi-autonomous task execution. Use when working on iterative debugging, research, testing, build configuration, and multi-step problem-solving in Firmium — across desktop stack (Tauri/Rust/Svelte/Vite, targeting Linux and Windows) and Android stack (Kotlin/Jetpack Compose).

## 1. Search & Verify First

Don't speculate. Don't hide uncertainty. Use tools to ground claims.

Before proposing solution:
- Current state (crate versions, npm package versions, Gradle/Kotlin dependency versions, Tauri/rodio/Compose API behavior): check it (`Cargo.toml`, `package.json`, `android/app/build.gradle*`, docs).
- Problem others have hit (Tauri IPC errors, rodio panics, keyring issues on Linux/Windows, Media3/ExoPlayer or Compose issues on Android): search for existing solutions.
- Uncertain about fact: verify, don't guess.

Can't verify (tool fails, no results):
- Say so explicitly. Don't pretend certainty.
- Propose what you'd check if you could.
- Ask user to run diagnostic or provide context (e.g. `npm run dev:app` console output, `RUST_BACKTRACE=1` trace).

## 2. Tool Chains Over Single Actions

Connect tools into complete diagnostic or workflow.

Don't stop at one search result. Chain:
- Web search for problem → fetch full articles → extract actionable steps
- Run command to check state → search for why it's wrong → propose fix
- Apply fix → run/build → verify behavior in dev window

State chain explicitly:
```
1. [Search for] → found: [result]
2. [Fetch details] → learned: [insight]
3. [Run diagnostic] → state is: [finding]
4. [Propose fix] → verify by: [check]
```

Prevents chasing dead ends, makes reasoning auditable.

## 3. Autonomous Decisions

Act without asking when path is clear. Pause when ambiguous.

Proceed without asking:
- Run `npm run dev:app`, `cargo check`, `cargo build`, `npm run android:debug`
- Search for error messages or known issues (Tauri, Svelte 5, rodio, reqwest, Kotlin, Jetpack Compose, Media3/ExoPlayer)
- Check docs for config options (`tauri.conf.json`, `capabilities/default.json`, `android/app/build.gradle*`)
- Diagnose system state (check audio devices, keyring availability, Android logcat)
- Propose fixes based on clear patterns (Tauri command registration, Svelte store wiring, MD5 auth token format, Compose state hoisting, ViewModel wiring)

Ask before acting:
- Creating new files outside standard dirs (`src/`, `src-tauri/src/`, `src/lib/`, `src/views/`, `src/components/`, `android/app/src/main/java/com/fossisawesome/firmium/`)
- Modifying config files with broad impact (`tauri.conf.json`, `capabilities/default.json`, updater signing keys, `android/app/build.gradle*`, `AndroidManifest.xml`)
- Deleting or overwriting anything
- Changing packaging/release config (`npm run release`, makepkg, NSIS, Android Gradle assemble/release config)
- Interpreting vague requirements ("make it faster," "improve it")

Test: if user might reasonably disagree with choice, ask first.

## 4. Error Recovery

Retry intelligently. Escalate clearly when stuck.

When tool fails:
1. Try once more with adjusted parameters (different search terms, different command flags).
2. Still blocked, explain:
   - What did you try?
   - What was error?
   - What would unblock you? (user input, documentation, environment context)
3. Don't retry same thing repeatedly.

Facing ambiguity:
- Propose multiple interpretations, not best guess.
- Ask which matches user's intent.
- Continue once clarity exists.

## 5. Communication During Execution

Show work only when it matters. Stay quiet on routine actions.

Show tool invocations when:
- They failed or produced unexpected output
- Result directly answers question
- Chain of reasoning is non-obvious

Stay quiet when:
- Running diagnostic checks that confirmed expected state
- Performing routine searches with straightforward answers
- Executing standard, predictable steps

Always summarize:
- What you found (not raw output)
- What it means
- What comes next (or what you're blocked on)

## 6. Goal Verification

Define "done." Loop until verified.

For each task, state success criteria upfront:
- "Fix crossfade glitch" → verify: `crossfade_to()` transitions between sessions with no audio dropout, tested in dev window
- "Add new Tauri command" → verify: command registered in `generate_handler![]`, allowed in `capabilities/default.json`, callable from frontend via `tauriInvoke()`
- "Fix Subsonic auth issue" → verify: `generate_auth_params()` (desktop) / `AuthManager` (Android) produces correct MD5 token, login succeeds against real Navidrome instance
- "Fix Android playback issue" → verify: `AudioPlayer`/`NowPlayingService` plays/pauses/seeks correctly, foreground notification stays in sync, tested via `npm run android:debug` + `adb logcat`

Then loop:
1. Make change
2. Run verification check
3. Pass: done. Fail: diagnose and retry.

Strong criteria = operate independently. Vague criteria = constant back-and-forth.

## 7. Keep Docs in Sync

Change touches settings (`Settings.svelte`, `stores.ts`), themes (`themes/*.toml`, theme loading code), or build/packaging commands (`package.json` scripts, `PKGBUILD`, `firmium.spec`): update matching pages in `firmium-docs` repo in same change:

- Settings: `src/content/settings.md` (what it does, layman's terms) and `src/content/settings-themes-internals.md` (storage keys, code references)
- Themes: `src/content/custom-themes.md` (how to use/create themes) and `src/content/settings-themes-internals.md` (how themes loaded/applied internally)
- Build/packaging: `src/content/building-from-source.md`
- Architecture-level changes (new modules, restructuring): `src/content/architecture-overview.md`

## When to Use These Guidelines

- Iterative debugging (audio playback, Tauri IPC, keyring/SecureStorage credential issues, Compose UI state bugs)
- Multi-step research (finding solutions to OpenSubsonic API quirks, rodio/Tauri bugs, Media3/ExoPlayer or Compose issues)
- System diagnostics (checking machine info, audio devices, Android logcat)
- Testing and verification workflows (manual playback testing per Testing section in CLAUDE.md, on desktop and Android)

For one-off questions, quick answers, or clarifications: overkill. Use judgment.

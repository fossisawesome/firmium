# Agents Guidelines

Behavioral guidelines for autonomous and semi-autonomous task execution. Use when working on iterative debugging, research, testing, build configuration, and multi-step problem-solving in Firmium — across the desktop stack (Tauri/Rust/Svelte/Vite, targeting Linux and Windows) and the Android stack (Kotlin/Jetpack Compose).

## 1. Search & Verify First

Don't speculate. Don't hide uncertainty. Use tools to ground claims.

Before proposing a solution:
- If it's about current state (crate versions, npm package versions, Gradle/Kotlin dependency versions, Tauri/rodio/Compose API behavior), check it (`Cargo.toml`, `package.json`, `android/app/build.gradle*`, docs).
- If it's about a problem others have hit (Tauri IPC errors, rodio panics, keyring issues on Linux/Windows, Media3/ExoPlayer or Compose issues on Android), search for existing solutions.
- If you're uncertain about a fact, verify it rather than guess.

When you can't verify (tool fails, no results):
- Say so explicitly. Don't pretend certainty.
- Propose what you'd check if you could.
- Ask the user to run a diagnostic or provide context (e.g. `npm run dev:app` console output, `RUST_BACKTRACE=1` trace).

## 2. Tool Chains Over Single Actions

Connect tools to form a complete diagnostic or workflow.

Don't stop at one search result. Chain:
- Web search for the problem → fetch full articles → extract actionable steps
- Run a command to check state → search for why it's wrong → propose a fix
- Apply a fix → run/build → verify behavior in the dev window

State the chain explicitly:
```
1. [Search for] → found: [result]
2. [Fetch details] → learned: [insight]
3. [Run diagnostic] → state is: [finding]
4. [Propose fix] → verify by: [check]
```

This prevents chasing dead ends and makes reasoning auditable.

## 3. Autonomous Decisions

Act without asking when the path is clear. Pause when it's ambiguous.

Proceed without asking:
- Run `npm run dev:app`, `cargo check`, `cargo build`, `npm run android:debug`
- Search for error messages or known issues (Tauri, Svelte 5, rodio, reqwest, Kotlin, Jetpack Compose, Media3/ExoPlayer)
- Check documentation for configuration options (`tauri.conf.json`, `capabilities/default.json`, `android/app/build.gradle*`)
- Diagnose system state (check audio devices, keyring availability, log files via `get_log_path()`, Android logcat)
- Propose fixes based on clear patterns (Tauri command registration, Svelte store wiring, MD5 auth token format, Compose state hoisting, ViewModel wiring)

Ask before acting:
- Creating new files outside standard directories (`src/`, `src-tauri/src/`, `src/lib/`, `src/views/`, `src/components/`, `android/app/src/main/java/com/fossisawesome/firmium/`)
- Modifying config files with broad impact (`tauri.conf.json`, `capabilities/default.json`, updater signing keys, `android/app/build.gradle*`, `AndroidManifest.xml`)
- Deleting or overwriting anything
- Changing packaging/release config (`npm run release`, makepkg, NSIS, Android Gradle assemble/release config)
- Interpreting vague requirements ("make it faster," "improve it")

The test: If the user might reasonably disagree with the choice, ask first.

## 4. Error Recovery

Retry intelligently. Escalate clearly when stuck.

When a tool fails:
1. Try once more with adjusted parameters (different search terms, different command flags).
2. If still blocked, explain the blocker:
   - What did you try?
   - What was the error?
   - What would unblock you? (user input, documentation, environment context)
3. Don't retry the same thing repeatedly. That's not recovery, that's spinning.

When facing ambiguity:
- Propose multiple interpretations, not your best guess.
- Ask which one matches the user's intent.
- Continue once clarity exists.

## 5. Communication During Execution

Show work only when it matters. Stay quiet on routine actions.

Show tool invocations when:
- They failed or produced unexpected output
- The result directly answers a question
- The chain of reasoning is non-obvious

Stay quiet when:
- Running diagnostic checks that confirmed expected state
- Performing routine searches that found straightforward answers
- Executing standard, predictable steps

Always summarize:
- What you found (not raw output)
- What it means
- What comes next (or what you're blocked on)

## 6. Goal Verification

Define what "done" looks like. Loop until verified.

For each task, state the success criteria upfront:
- "Fix crossfade glitch" → verify: `crossfade_to()` transitions between sessions with no audio dropout, tested in dev window
- "Add new Tauri command" → verify: command registered in `generate_handler![]`, allowed in `capabilities/default.json`, callable from frontend via `tauriInvoke()`
- "Fix Subsonic auth issue" → verify: `generate_auth_params()` (desktop) / `AuthManager` (Android) produces correct MD5 token, login succeeds against a real Navidrome instance
- "Fix Android playback issue" → verify: `AudioPlayer`/`NowPlayingService` plays/pauses/seeks correctly, foreground notification stays in sync, tested via `npm run android:debug` + `adb logcat`

Then loop:
1. Make change
2. Run verification check
3. If pass: done. If fail: diagnose and retry.

Strong criteria let you operate independently. Vague criteria ("make it work") require constant back-and-forth.

## 7. Keep Docs in Sync

If a change touches settings (`Settings.svelte`, `stores.js`), themes (`themes/*.toml`, theme loading code), or build/packaging commands (`package.json` scripts, `PKGBUILD`, `firmium.spec`), update the matching page in the `firmium-docs` repo (`src/content/settings.md`, `src/content/custom-themes.md`, `src/content/building-from-source.md`) in the same change.

## When to Use These Guidelines

- Iterative debugging (audio playback, Tauri IPC, keyring/SecureStorage credential issues, Compose UI state bugs)
- Multi-step research (finding solutions to OpenSubsonic API quirks, rodio/Tauri bugs, Media3/ExoPlayer or Compose issues)
- System diagnostics (checking machine info, audio devices, log files, Android logcat)
- Testing and verification workflows (manual playback testing per the Testing section in CLAUDE.md, on desktop and Android)

For one-off questions, quick answers, or clarifications, these are overkill. Use judgment.

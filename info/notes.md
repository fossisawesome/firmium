# Before testing
- Do a /codereview with Claude Sonnet 4.6
- Fix the issues and bugs
- Update verisons in Cargo.toml | tauri.conf.json | PKGBUILD | package.json | package-lock.json

---

# Testing
- Begin building the app with "npm run build:arch".
- Run "sudo pacman -R firmium-desktop"
- Install, "sudo pacman -U src-tauri/target/release/bundle/arch/*.pkg.tar.zst".
- Stress test the app for 1 hour before moving on.
- Do a manual code review of the app.

---

# Push to Git
- Run "git add ."
- Then run "git commit -m "MyCommitMessage".
- Run "git push origin main" to send everything to the repo.
- Run "git status" to confirm it worked.

---

# Misc:
- Will you ever add Windows support?: Yes, but I want to add all the features I want and polish the app before that.
- Will you ever add MacOS support?: Same as Windows, it will happen sometime but I want to have the app polished before that.
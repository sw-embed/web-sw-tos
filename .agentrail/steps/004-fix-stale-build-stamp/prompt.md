Fix the frozen build provenance in the footer.

Defect: build.rs emits only 'cargo:rerun-if-changed=build.rs'. Once any rerun-if-changed is emitted, cargo re-runs the build script ONLY for those paths, so BUILD_SHA / BUILD_HOST / BUILD_TIMESTAMP are captured on the first build and cached forever. The deployed footer showed 'Build Commit f6502ed / Build Time 2026-08-28T16:37:02Z' -- the bootstrap commit -- after three later build-pages.sh runs. sw-checklist still passes because it only looks for the labels, not the values, so the gate cannot catch this.

Fix: also watch the git ref so the script re-runs when HEAD moves. Watch .git/HEAD, read it, and if it holds 'ref: <path>' watch .git/<path> as well. build.rs runs with the package root as CWD so these paths resolve.

Acceptance: rebuild pages/ and confirm the baked timestamp and short SHA match the actual HEAD at build time, not the first build. Verify by inspecting the emitted wasm, not by assuming. Keep build.rs within sw-checklist limits (functions <= 25 LOC, module <= 4 functions). clippy and fmt clean; sw-checklist stays 16/0/0.
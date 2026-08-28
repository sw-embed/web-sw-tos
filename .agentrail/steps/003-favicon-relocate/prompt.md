Relocate the desktop-icon favicon.ico out of the repo root and make the live demo use it.

Mike supplied a 256x256 desktop-monitor favicon.ico at the repo root, replacing the generated 16x16 placeholder. Move the real file to static/ and reference it from index.html so Trunk copies it into the bundle.

Constraint discovered while doing this: sw-checklist's 'favicon.ico' check requires the file at the REPO ROOT, and fails if it is only in a subdirectory. A root symlink (favicon.ico -> static/favicon.ico) satisfies the checker while keeping the real asset out of the root. Verify sw-checklist stays at 16 passed / 0 failed / 0 warnings.

Deliverables: static/favicon.ico (the real file, git mv so history follows), a root symlink, index.html referencing static/favicon.ico, and a rebuilt pages/ carrying the new icon.

Acceptance: trunk build emits the 256x256 icon (not the old 1150-byte placeholder); the icon is confirmed loading in a browser tab; sw-checklist 16/0/0; clippy and fmt clean.
Rewrite README.md to the Software Wrighter house structure and normalize the copyright string.

Structure (see ../sw-cor24-emulator/README.md for the house style): title, blurb, bold Live Demo link, screen capture, intro, links to docs/, a developer section, MIT license note near the end, and 'Copyright (c) 2026 Michael A Wright' at the very end.

Also: the canonical string across all sibling repos is 'Copyright (c) 2026 Michael A Wright' with NO period after the A. COPYRIGHT and src/footer.rs currently write 'Michael A. Wright'. Fix both. LICENSE is already correct.

Capture a real screenshot of the running demo into images/ and reference it from the README.

Acceptance: sw-markdown-checker passes on README.md; sw-checklist stays at 16 passed / 0 failed / 0 warnings; clippy and fmt clean.
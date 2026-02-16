# Documentation Reorganization Notice

**Date:** 2026-02-16  
**Action:** All markdown documentation files have been moved from the project root to organized subdirectories within `docs/`

## What Changed

Previously scattered markdown files in the root directory have been organized into:

```
docs/
├── project-management/     # AGENTS.md, STATUS.md, TODO.md, WEEKLY_PROGRESS_TRACKER.md
├── architecture/           # spec.md, KIMI_CONTEXT.md, urs.md
├── implementation-plans/   # PHASE2*, VFAT32*, RASPBERRY_PI_5_PORTING_PLAN.md
├── hardware/              # Hardware planning and finance documents
├── build-results/         # buglist.md, buglist-x64.md, build_summary.md
├── drivers/               # Driver documentation (already organized)
├── filesystems/           # Filesystem documentation (already organized)
└── gui-input/             # GUI/input documentation (already organized)
```

## Files That Remain in Root

- **README.md** - Main project README (stays for GitHub visibility)
- **Cargo.toml** - Rust project configuration
- **Makefile** - Build system
- **Build scripts** - `.sh` and `.bat` files
- **Source code directories** - `kernel/`, `bootloader/`, `system/`, etc.
- **Configuration files** - `.json`, `.toml`, `.txt` files

## How to Access Documentation

1. **Browse:** Navigate to `docs/` and use the subdirectories
2. **Search:** Use `find docs/ -name "*.md"` or `grep -r "keyword" docs/`
3. **Reference:** See `docs/README.md` for complete directory map

## For Agents/Automation

All agents and automation scripts should:
- Search for documentation in `docs/` directory first
- Use `find` commands to locate specific files
- Check `docs/README.md` for navigation help

## Notes
- This reorganization improves project structure and maintainability
- Internal links in documentation may need updating if they reference moved files
- Future documentation should be added to appropriate `docs/` subdirectories
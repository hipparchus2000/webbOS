# webbOS Documentation

This directory contains all documentation for the webbOS project, organized by category.

## 📁 Directory Structure

### `project-management/`
- **AGENTS.md** - Agent coordination and workflow guidelines
- **STATUS.md** - Current project status and progress
- **TODO.md** - Task list and future work
- **WEEKLY_PROGRESS_TRACKER.md** - Weekly progress tracking

### `architecture/`
- **spec.md** - Technical specification and architecture design
- **KIMI_CONTEXT.md** - Context for Kimi AI agent interactions
- **urs.md** - User requirements and specifications

### `implementation-plans/`
- **PHASE2_IMPLEMENTATION_REPORT.md** - Phase 2 implementation report
- **RASPBERRY_PI_5_PORTING_PLAN.md** - ARM64/Raspberry Pi 5 porting plan
- **VFAT32-IMPLEMENTATION-PLAN.md** - VFAT32 filesystem implementation plan
- **VFAT32-WRITE-PROJECT-START.md** - VFAT32 write project kickoff

### `hardware/`
- **HARDWARE_ACQUISITION_PLAN.md** - Hardware purchase and setup plan
- **CONSOLIDATED_RECOMMENDATIONS.md** - Hardware recommendations
- **HARDWARE_SETUP_TIMELINE.md** - Hardware setup schedule
- **PHASE2_RESOURCE_PLAN.md** - Phase 2 resource allocation
- **PHASE2_STARTUP_CHECKLIST.md** - Startup checklist
- **PHASE2_SUMMARY_REPORT.md** - Phase 2 summary
- **TOOLCHAIN_RECOMMENDATIONS.md** - Development toolchain recommendations
- **WEEBOS_RASPBERRY_PI_5_BUDGET.xlsx.md** - Budget spreadsheet (markdown version)

### `build-results/`
- **buglist.md** - ARM64 build bug analysis
- **buglist-x64.md** - X64 build bug analysis
- **build_summary.md** - Build execution summary

### `drivers/`
- **README.md** - Drivers overview
- **ethernet.md** - Ethernet driver documentation
- **gpio.md** - GPIO driver documentation
- **hal.md** - Hardware Abstraction Layer documentation
- **uart.md** - UART driver documentation
- **usb.md** - USB driver documentation

### `filesystems/`
- **README.md** - Filesystems overview
- **SD_CARD_DRIVER.md** - SD card driver documentation
- **week4-filesystem-integration-research.md** - Filesystem integration research

### `gui-input/`
- **Week5_Executive_Summary.md** - GUI/input executive summary
- **Week5_GUI_Input_Subsystems_Research.md** - GUI/input research
- **implementation-plan.md** - GUI/input implementation plan
- **implementation/** - Detailed implementation documents

## 🔍 How to Find Documents

### For Humans:
- Browse the folder structure above
- Use the table of contents in this README

### For Agents/Automation:
```bash
# Find all markdown files
find docs/ -name "*.md"

# Search for specific content
grep -r "keyword" docs/

# List files by category
ls docs/project-management/
ls docs/architecture/
# etc.
```

## 📝 Notes
- All documentation has been moved from the project root to this organized structure
- The main `README.md` remains in the project root for GitHub visibility
- Internal links may need updating if they reference moved files
- New documentation should be added to the appropriate subdirectory
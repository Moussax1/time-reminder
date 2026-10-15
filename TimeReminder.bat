@echo off
cd /d "%~dp0"
if exist "target\release\time-reminder.exe" (
    start "" "target\release\time-reminder.exe"
) else (
    echo [ERROR] target\release\time-reminder.exe not found!
    echo Please run "cargo build --release" first.
    pause
)

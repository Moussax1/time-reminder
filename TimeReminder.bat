@echo off
cd /d "%~dp0"
if exist "time-reminder.exe" (
    start "" "time-reminder.exe"
) else if exist "target\release\time-reminder.exe" (
    start "" "target\release\time-reminder.exe"
) else (
    echo [ERROR] time-reminder.exe not found!
    echo Please ensure time-reminder.exe is in this folder, or run "cargo build --release".
    pause
)

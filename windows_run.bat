@echo off
REM Change the current directory to the location of this batch file's parent folder (your project root)
cd /d "%~dp0"

REM Run the Rust project using Cargo
echo Running Cargo project...
cargo run

REM Keep the window open after execution to see the output
echo.
echo Press any key to exit...
pause > nul

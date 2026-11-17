@echo off
rem Set Fractique up on Windows with one command: setup.cmd [--skip-tests]
rem It installs Rust when missing (rustup, into your user folder) and the Microsoft C++ Build Tools that Rust links
rem with, builds the command line tool and runs the tests, and ends with the command that renders your first image.
rem Safe to run again.
setlocal
cd /d "%~dp0"

where cargo >nul 2>&1 && goto have_cargo
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" goto add_cargo
echo [setup] Installing Rust (rustup)...
powershell -NoProfile -ExecutionPolicy Bypass -Command "Invoke-WebRequest -UseBasicParsing https://win.rustup.rs/x86_64 -OutFile '%TEMP%\rustup-init.exe'" || goto failed
"%TEMP%\rustup-init.exe" -y --profile minimal --default-toolchain none || goto failed
del "%TEMP%\rustup-init.exe" >nul 2>&1
:add_cargo
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
:have_cargo

rem Rust on Windows links with the Microsoft C++ Build Tools; vswhere says whether they are installed.
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
set "VCTOOLS="
if not exist "%VSWHERE%" goto need_tools
for /f "usebackq delims=" %%i in (`"%VSWHERE%" -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VCTOOLS=%%i"
if defined VCTOOLS goto build
:need_tools
where winget >nul 2>&1 || goto no_winget
echo [setup] Installing the Microsoft C++ Build Tools (several GB, Windows asks for permission)...
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-package-agreements --accept-source-agreements --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" || goto failed
goto build
:no_winget
echo [setup] The Microsoft C++ Build Tools are missing and winget is not available to install them.
echo [setup] Install "Desktop development with C++" from https://visualstudio.microsoft.com/visual-cpp-build-tools/
echo [setup] and run this again.
exit /b 1

:build
rem The first cargo command also installs the Rust version pinned in rust-toolchain.toml.
echo [setup] Building (the first build takes a minute or two)...
cargo build --release --locked || goto failed
if "%~1"=="--skip-tests" goto done
echo [setup] Running the tests...
cargo test --release --locked --quiet || goto failed

:done
echo [setup] Done. Render your first image:
echo [setup]   target\release\fractique.exe render --preset aurora-spiral -o aurora.png
echo [setup] More: target\release\fractique.exe --help, and docs\usage.md
rem This cmd session keeps the PATH that finds cargo.
endlocal & set "PATH=%PATH%" & exit /b 0

:failed
echo [setup] Setup stopped: see the message above. It is safe to run setup.cmd again.
exit /b 1

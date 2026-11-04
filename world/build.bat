@echo off
REM Build Dreamride's world: the engine as luminesce.dll (cargo), then the native host (MSVC),
REM into world\bin. No project or solution files. Needs Rust, MSVC (Desktop development with
REM C++) and the Windows 10/11 SDK (for d3d12, dxgi, xaudio2 and dxcompiler.dll).
REM
REM   build.bat        the engine, then the host, into world\bin
REM   build.bat test   the UI, equation-layout, history and water tests (world\tests, no GPU, no window), built and run;
REM                    then bin\luminesce-world.exe --history-test (the history against the engine) and --camera-seam-test
REM                    (the camera across a stream's seams) when it is built
setlocal
cd /d "%~dp0"
if /i "%~1"=="test" goto :msvc

REM --- the engine ---------------------------------------------------------------------------
where cargo >nul 2>nul || set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
pushd ..
cargo build --release --lib
if errorlevel 1 ( popd & echo. & echo BUILD FAILED: cargo & exit /b 1 )
popd

:msvc
REM --- MSVC (vswhere first, then the known 2022 paths), as in Dreamzone-RT's build.bat ------
set "PF=%ProgramFiles%"
set "PFX86=%ProgramFiles(x86)%"
set "VCVARS="
set "VSWHERE=%PFX86%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" set "VSWHERE=%PF%\Microsoft Visual Studio\Installer\vswhere.exe"
set "VSPATH="
if not exist "%VSWHERE%" goto :no_vswhere
for /f "usebackq tokens=*" %%i in (`call "%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2^>nul`) do set "VSPATH=%%i"
:no_vswhere
if defined VSPATH if exist "%VSPATH%\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%VSPATH%\VC\Auxiliary\Build\vcvars64.bat"
if not defined VCVARS for %%e in (Community Professional Enterprise BuildTools) do (
  if not defined VCVARS if exist "%PF%\Microsoft Visual Studio\2022\%%e\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%PF%\Microsoft Visual Studio\2022\%%e\VC\Auxiliary\Build\vcvars64.bat"
  if not defined VCVARS if exist "%PFX86%\Microsoft Visual Studio\2022\%%e\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%PFX86%\Microsoft Visual Studio\2022\%%e\VC\Auxiliary\Build\vcvars64.bat"
)
if not defined VCVARS (
  echo. & echo BUILD FAILED: could not find vcvars64.bat ^(MSVC x64 tools^).
  exit /b 1
)
call "%VCVARS%" >nul

if not exist build mkdir build
if not exist bin mkdir bin
if /i "%~1"=="test" goto :test

REM --- Dear ImGui (vendored in src\imgui, MIT): compiled once, rebuilt only when missing ------
if not exist build\imgui\imgui_widgets.obj (
  if not exist build\imgui mkdir build\imgui
  cl /nologo /c /EHsc /O2 /W3 /std:c++17 /MP /Fo:build\imgui\ src\imgui\imgui.cpp src\imgui\imgui_draw.cpp ^
     src\imgui\imgui_tables.cpp src\imgui\imgui_impl_dx12.cpp src\imgui\imgui_impl_win32.cpp src\imgui\imgui_widgets.cpp
  if errorlevel 1 ( echo. & echo BUILD FAILED: cl imgui & exit /b 1 )
)

REM --- the icon, embedded so the window, the taskbar and the shortcut show it ---------------
rc /nologo /fo build\world.res src\world.rc
if errorlevel 1 ( echo. & echo BUILD FAILED: rc & exit /b 1 )

cl /nologo /EHsc /O2 /W3 /std:c++17 /utf-8 /Fo:build\ /Fe:bin\luminesce-world.exe src\main.cpp build\imgui\*.obj build\world.res ^
   /link ..\target\release\luminesce.dll.lib d3d12.lib dxgi.lib dxguid.lib xaudio2.lib ole32.lib user32.lib gdi32.lib winmm.lib ^
   /SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup
if errorlevel 1 ( echo. & echo BUILD FAILED: cl & exit /b 1 )

REM --- runtime files next to the exe: the engine, the shader compiler, the shaders -----------
copy /Y ..\target\release\luminesce.dll bin\ >nul
copy /Y shaders\world.hlsl bin\ >nul
set "SDKBIN="
for /f "delims=" %%v in ('dir /b /ad /o-n "%PFX86%\Windows Kits\10\bin\10.*" 2^>nul') do (
  if not defined SDKBIN if exist "%PFX86%\Windows Kits\10\bin\%%v\x64\dxcompiler.dll" set "SDKBIN=%PFX86%\Windows Kits\10\bin\%%v\x64"
)
if not defined SDKBIN ( echo. & echo BUILD FAILED: no dxcompiler.dll in the Windows SDK & exit /b 1 )
copy /Y "%SDKBIN%\dxcompiler.dll" bin\ >nul
copy /Y "%SDKBIN%\dxil.dll" bin\ >nul
echo. & echo BUILD OK -^> world\bin\luminesce-world.exe
exit /b 0

:test
cl /nologo /EHsc /O2 /W3 /std:c++17 /utf-8 /Fo:build\ /Fe:build\ui_logic_test.exe tests\ui_logic_test.cpp
if errorlevel 1 ( echo. & echo TEST BUILD FAILED: cl & exit /b 1 )
build\ui_logic_test.exe
if errorlevel 1 ( echo. & echo TESTS FAILED & exit /b 1 )
cl /nologo /EHsc /O2 /W3 /std:c++17 /utf-8 /Fo:build\ /Fe:build\math_layout_test.exe tests\math_layout_test.cpp
if errorlevel 1 ( echo. & echo TEST BUILD FAILED: cl math_layout_test & exit /b 1 )
build\math_layout_test.exe
if errorlevel 1 ( echo. & echo TESTS FAILED & exit /b 1 )
cl /nologo /EHsc /O2 /W3 /std:c++17 /utf-8 /Fo:build\ /Fe:build\history_test.exe tests\history_test.cpp
if errorlevel 1 ( echo. & echo TEST BUILD FAILED: cl history_test & exit /b 1 )
build\history_test.exe
if errorlevel 1 ( echo. & echo TESTS FAILED & exit /b 1 )
cl /nologo /EHsc /O2 /W3 /std:c++17 /utf-8 /Fo:build\ /Fe:build\water_test.exe tests\water_test.cpp
if errorlevel 1 ( echo. & echo TEST BUILD FAILED: cl water_test & exit /b 1 )
build\water_test.exe
if errorlevel 1 ( echo. & echo TESTS FAILED & exit /b 1 )
REM The history against the real engine (records a stream, replays it, compares the audio), when the world is built.
if exist bin\luminesce-world.exe (
  bin\luminesce-world.exe --history-test
  if errorlevel 1 ( echo. & echo TESTS FAILED: --history-test & exit /b 1 )
  bin\luminesce-world.exe --camera-seam-test
  if errorlevel 1 ( echo. & echo TESTS FAILED: --camera-seam-test & exit /b 1 )
)
echo. & echo TESTS OK

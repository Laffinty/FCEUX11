# FCEUX DLL Dependencies

## Quick Start

Run the launch script to start FCEUX:
```
build\src\dependencies\run_fceux.bat
```

Or manually set PATH and run:
```batch
set PATH=D:\msys64\mingw64\bin;D:\msys64\usr\bin;%PATH%
cd build\src
fceux.exe
```

## DLL Dependency Analysis

### Direct Dependencies (7 DLLs)
| DLL | Description |
|-----|-------------|
| libarchive-13.dll | Archive library (ZIP, 7z, etc.) |
| libgcc_s_seh-1.dll | GCC runtime (exception handling) |
| libstdc++-6.dll | C++ standard library |
| Qt5Core.dll | Qt5 core module |
| Qt5Gui.dll | Qt5 GUI module |
| Qt5Widgets.dll | Qt5 widgets module |
| SDL2.dll | Simple DirectMedia Layer |

### System DLLs (7 DLLs - Built into Windows)
| DLL | Description |
|-----|-------------|
| KERNEL32.dll | Windows kernel |
| USER32.dll | Windows user interface |
| msvcrt.dll | Microsoft C runtime |
| MSVFW32.dll | Video for Windows |
| OPENGL32.dll | OpenGL |
| WSOCK32.dll | Windows sockets |
| hhctrl.ocx | HTML Help |

### Indirect Dependencies (17 MinGW DLLs)
| DLL | Description |
|-----|-------------|
| libb2-1.dll | BLAKE2 hash library |
| libbz2-1.dll | bzip2 compression |
| libcrypto-3-x64.dll | OpenSSL cryptography |
| libdouble-conversion.dll | Double precision conversion |
| libexpat-1.dll | XML parser |
| libharfbuzz-0.dll | Text shaping engine |
| libiconv-2.dll | Character encoding conversion |
| libicuin78.dll | ICU (Unicode support) |
| libicuuc78.dll | ICU (Unicode support) |
| liblz4.dll | LZ4 compression |
| liblzma-5.dll | LZMA compression |
| libmd4c.dll | Markdown parser |
| libpcre2-16-0.dll | Regular expressions |
| libpng16-16.dll | PNG image format |
| libwinpthread-1.dll | POSIX threads |
| libzstd.dll | Zstandard compression |
| zlib1.dll | zlib compression |

## Complete DLL List

All 24 MinGW DLLs (place in same folder as fceux.exe):
```
libarchive-13.dll
libb2-1.dll
libbz2-1.dll
libcrypto-3-x64.dll
libdouble-conversion.dll
libexpat-1.dll
libgcc_s_seh-1.dll
libharfbuzz-0.dll
libiconv-2.dll
libicuin78.dll
libicuuc78.dll
liblz4.dll
liblzma-5.dll
libmd4c.dll
libpcre2-16-0.dll
libpng16-16.dll
libstdc++-6.dll
libwinpthread-1.dll
libzstd.dll
Qt5Core.dll
Qt5Gui.dll
Qt5Widgets.dll
SDL2.dll
zlib1.dll
```

## Rebuild Script

To rebuild from scratch:

```powershell
# Clean and rebuild
Remove-Item -Path build -Recurse -Force
New-Item -ItemType Directory -Force -Path build

$env:PATH = "D:\msys64\mingw64\bin;D:\msys64\usr\bin;$env:PATH"
$env:MINGW_PREFIX = "D:\msys64\mingw64"

cd build
cmake .. -G "MinGW Makefiles" -DCMAKE_CXX_FLAGS="-DPSS_STYLE=2" -DCMAKE_C_FLAGS="-DPSS_STYLE=2"
mingw32-make -j4

# Copy dependencies
cd ..
& .\scripts\copy_dependencies.ps1 -Executable "build\src\fceux.exe" -OutputDir "build\src\dependencies"

# Run
& .\build\src\dependencies\run_fceux.bat
```

## Auto-Copy Script Usage

The `scripts\copy_dependencies.ps1` script can analyze any MinGW executable:

```powershell
.\scripts\copy_dependencies.ps1 -Executable "path\to\your_app.exe" -OutputDir "path\to\output_folder"
```

This will:
1. Analyze direct DLL dependencies using objdump
2. Copy all MinGW DLLs to the output folder
3. Analyze indirect dependencies and copy them too
4. Generate a launch script
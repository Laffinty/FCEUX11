@echo off
set "LOGFILE=%TEMP%\fceux_build.log"
/d/msys64/usr/bin/bash.exe -l -c "export PATH='/c/Users/ikrx2/.cargo/bin:$PATH' && cd /c/Users/ikrx2/Desktop/project/FCEUX11 && rm -rf build && mkdir -p build && cd build && cmake .. -G 'MSYS Makefiles' -DCMAKE_BUILD_TYPE=Release && make -j4" > %LOGFILE% 2>&1
type %LOGFILE%
exit /b %ERRORLEVEL%

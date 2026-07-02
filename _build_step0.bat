@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64
cd /d D:\Project\FCEUX11\build
nmake /f Makefile /nologo
exit /b %ERRORLEVEL%

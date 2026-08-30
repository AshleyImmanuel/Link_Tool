@echo off
call "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
set LIB=C:\Program Files (x86)\Windows Kits\10\Lib\10.0.19041.0\um\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.19041.0\ucrt\x64;%LIB%
cargo %*

@echo off
setlocal
cd /d "%~dp0"
target\release\edge-notifier.exe notify %*

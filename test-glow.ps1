# Trigger Screen Glow Notifier via Local API
param (
    [string]$Color = "#FF2222",
    [double]$Duration = 5.0,
    [int]$Width = 45
)

$encodedColor = [System.Uri]::EscapeDataString($Color)
$url = "http://127.0.0.1:4545/notify?color=$encodedColor&duration=$Duration&width=$Width"

try {
    $resp = Invoke-RestMethod -Uri $url -Method Get -TimeoutSec 2
    Write-Host "✅ Notification glow triggered: $url" -ForegroundColor Green
} catch {
    Write-Host "⚠️ Notifier service is not running. Starting it now..." -ForegroundColor Yellow
    Start-Process ".\target\release\edge-notifier.exe"
    Start-Sleep -Milliseconds 800
    Invoke-RestMethod -Uri $url -Method Get
    Write-Host "✅ Notifier started and glow triggered!" -ForegroundColor Green
}

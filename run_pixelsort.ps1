# Raspberry Pi Pixel Sorter - Auto-update launcher script (Windows version)
# This script updates the app from git and runs it

$APP_DIR = "C:\Users\joel\Pixelsort"
$REPO_URL = "https://github.com/Vaghabund/Pixelsort.git"

Set-Location $APP_DIR

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Raspberry Pi Pixel Sorter - Starting..." -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# Check for internet connectivity
try {
    $null = Test-Connection -ComputerName github.com -Count 1 -ErrorAction Stop
    Write-Host "Internet connected. Checking for updates..." -ForegroundColor Green
    
    # Fetch latest changes
    git fetch origin main
    
    # Check if we're behind
    $LOCAL = git rev-parse HEAD
    $REMOTE = git rev-parse origin/main
    
    if ($LOCAL -ne $REMOTE) {
        Write-Host "Updates found! Pulling changes..." -ForegroundColor Yellow
        git pull origin main
    } else {
        Write-Host "Already up to date." -ForegroundColor Green
    }
} catch {
    Write-Host "No internet connection. Skipping update check." -ForegroundColor Yellow
}

Write-Host "Starting Pixel Sorter..." -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# Run the application (release mode for better performance)
# cargo run will automatically rebuild only if needed
cargo run --release

# If app exits, wait a moment before this script ends
Write-Host ""
Write-Host "Application closed." -ForegroundColor Cyan
Start-Sleep -Seconds 2

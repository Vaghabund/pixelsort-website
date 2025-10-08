# Quick syntax check script for Raspberry Pi project
# This checks Rust files for syntax errors without full compilation

Write-Host "🦀 Checking Rust syntax for Pi project..." -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Check each Rust source file individually for syntax errors
$files = @(
    "src/main.rs",
    "src/ui.rs",
    "src/pixel_sorter.rs",
    "src/camera_controller.rs",
    "src/image_processor.rs",
    "src/config.rs"
)

$hasErrors = $false

foreach ($file in $files) {
    if (Test-Path $file) {
        Write-Host "Checking $file..." -ForegroundColor Yellow
        
        # Run rustc with syntax-only checking
        # We'll see unresolved import errors but catch syntax/delimiter issues
        $output = rustc --crate-type lib $file --edition 2021 2>&1 | Out-String
        
        # Filter for syntax errors (mismatched delimiters, unclosed blocks, etc.)
        $syntaxErrors = $output | Select-String -Pattern "mismatched|unclosed|expected.*found|unexpected" -AllMatches
        
        if ($syntaxErrors) {
            Write-Host "  ❌ Syntax errors found in $file" -ForegroundColor Red
            $syntaxErrors | ForEach-Object { Write-Host "     $_" -ForegroundColor Red }
            $hasErrors = $true
        } else {
            Write-Host "  ✅ No syntax errors in $file" -ForegroundColor Green
        }
    }
}

Write-Host ""
if (-not $hasErrors) {
    Write-Host "✅ All files passed syntax check!" -ForegroundColor Green
    Write-Host "Note: Import errors are expected and ignored - only syntax errors are reported." -ForegroundColor Gray
    exit 0
} else {
    Write-Host "❌ Syntax errors found! Fix them before pushing to Pi." -ForegroundColor Red
    exit 1
}

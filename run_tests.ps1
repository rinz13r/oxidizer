# Oxidizer Test Runner
# Usage:
#   ./run_tests.ps1              # Run all tests (unit + E2E)
#   ./run_tests.ps1 unit         # Cargo unit tests only
#   ./run_tests.ps1 dotnet       # C# E2E tests only
#   ./run_tests.ps1 python       # Python E2E tests only
#   ./run_tests.ps1 e2e          # Both C# and Python E2E tests
#   ./run_tests.ps1 dotnet python # Multiple targets

param(
    [Parameter(Position = 0, ValueFromRemainingArguments)]
    [string[]]$Targets
)

$ErrorActionPreference = "Stop"

if (-not $Targets) { $Targets = @("all") }

$runUnit   = $Targets -contains "all" -or $Targets -contains "unit"
$runDotnet = $Targets -contains "all" -or $Targets -contains "dotnet" -or $Targets -contains "e2e"
$runPython = $Targets -contains "all" -or $Targets -contains "python" -or $Targets -contains "e2e"
$runE2E    = $runDotnet -or $runPython

# Always build core crates
Write-Host "=== Building core oxidizer crates ===" -ForegroundColor Cyan
cargo build
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

if ($runE2E) {
    Write-Host "`n=== Building rust_lib DLL ===" -ForegroundColor Cyan
    cargo build -p rust_lib
    if ($LASTEXITCODE -ne 0) { throw "cargo build -p rust_lib failed" }

    Write-Host "`n=== Generating bindings ===" -ForegroundColor Cyan
    cargo build -p bindings-generator
    if ($LASTEXITCODE -ne 0) { throw "cargo build -p bindings-generator failed" }
}

if ($runUnit) {
    Write-Host "`n=== Running Cargo unit tests ===" -ForegroundColor Cyan
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
}

if ($runDotnet) {
    Write-Host "`n=== Running C# (xUnit) tests ===" -ForegroundColor Cyan
    dotnet test tests/e2e/dotnet/DotnetTests.csproj --verbosity normal
    if ($LASTEXITCODE -ne 0) { throw "dotnet test failed" }
}

if ($runPython) {
    Write-Host "`n=== Running Python (pytest) tests ===" -ForegroundColor Cyan
    python -m pytest tests/e2e/python/ -v
    if ($LASTEXITCODE -ne 0) { throw "pytest failed" }
}

Write-Host "`n=== All selected tests passed ===" -ForegroundColor Green

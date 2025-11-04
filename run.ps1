# Store the original directory
$originalDir = Get-Location

try {
    # Build Rust library
    Set-Location rust_lib
    cargo build
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed"
    }
    
    # Run .NET application
    Set-Location ../DotnetApp
    dotnet run
    if ($LASTEXITCODE -ne 0) {
        throw "Dotnet run failed"
    }
}
catch {
    Write-Error "Script failed: $_"
}
finally {
    # Always return to the original directory
    Set-Location $originalDir
}

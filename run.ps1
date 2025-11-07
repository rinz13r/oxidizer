# Store the original directory
$originalDir = Get-Location

try {
    # Build Rust library
    cargo build
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed"
    }
    
    # Run .NET application
    dotnet run --project DotnetApp\DotnetApp.csproj
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

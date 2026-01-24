# Store the original directory
$originalDir = Get-Location

try {
    cargo build
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed"
    }

    # Build rust_lib crate (produces rust_lib.dll)
    cargo build -p rust_lib
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build for rust_lib failed"
    }

    # Build bindings-generator to generate Generated.cs
    cargo build -p bindings-generator
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build for bindings-generator failed"
    }

    # Build .NET application
    dotnet build examples\DotnetApp\DotnetApp.csproj
    if ($LASTEXITCODE -ne 0) {
        throw "Dotnet build failed"
    }
    
    # Run .NET application
    dotnet run --project examples\DotnetApp\DotnetApp.csproj --no-build
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

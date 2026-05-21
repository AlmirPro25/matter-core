param(
    [string]$OutputPath = "dist\matter-core-beta-setup.exe",
    [string]$ZipPath = "dist\matter-core-windows-x64.zip",
    [string]$ChecksumJsonPath = "dist\release-checksums.json",
    [string]$Sha256Path = "dist\SHA256SUMS.txt",
    [string]$InstallerPath = "scripts\install-release-zip.ps1",
    [string]$LauncherPath = "scripts\install-matter-beta.cmd"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

foreach ($path in @($ZipPath, $ChecksumJsonPath, $Sha256Path, $InstallerPath, $LauncherPath)) {
    if (-not (Test-Path $path -PathType Leaf)) {
        throw "Required setup input not found: $path"
    }
}

$cscCandidates = @(
    "C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe",
    "C:\Windows\Microsoft.NET\Framework\v4.0.30319\csc.exe"
)
$csc = $cscCandidates | Where-Object { Test-Path $_ -PathType Leaf } | Select-Object -First 1
if (-not $csc) {
    throw "csc.exe not found. Install .NET Framework SDK or Windows SDK."
}

$outputFullPath = [System.IO.Path]::GetFullPath($OutputPath)
$outputParent = Split-Path -Parent $outputFullPath
if ($outputParent) {
    New-Item -ItemType Directory -Force $outputParent | Out-Null
}

$buildDir = Join-Path ([System.IO.Path]::GetTempPath()) ("matter_setup_build_" + [guid]::NewGuid().ToString("N"))
$sourcePath = Join-Path $buildDir "MatterCoreBetaSetup.cs"

try {
    New-Item -ItemType Directory -Force $buildDir | Out-Null

    $source = @'
using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;

internal static class MatterCoreBetaSetup
{
    private static readonly string[] Files = new[]
    {
        "matter-core-windows-x64.zip",
        "release-checksums.json",
        "SHA256SUMS.txt",
        "install-release-zip.ps1",
        "install-matter-beta.cmd"
    };

    public static int Main(string[] args)
    {
        Console.Title = "Matter Core Beta Setup";
        Console.WriteLine("Matter Core Beta Setup");
        Console.WriteLine();

        string extractRoot = Path.Combine(Path.GetTempPath(), "matter_core_beta_setup_" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(extractRoot);

        try
        {
            Assembly asm = Assembly.GetExecutingAssembly();
            foreach (string file in Files)
            {
                using (Stream input = asm.GetManifestResourceStream(file))
                {
                    if (input == null)
                    {
                        throw new InvalidOperationException("Embedded setup resource missing: " + file);
                    }

                    string target = Path.Combine(extractRoot, file);
                    using (FileStream output = File.Create(target))
                    {
                        input.CopyTo(output);
                    }
                }
            }

            string launcher = Path.Combine(extractRoot, "install-matter-beta.cmd");
            string forwardedArgs = string.Join(" ", Array.ConvertAll(args, QuoteArgument));
            string cmdArgs = "/c \"\"" + launcher + "\" " + forwardedArgs + "\"";

            ProcessStartInfo psi = new ProcessStartInfo("cmd.exe", cmdArgs);
            psi.WorkingDirectory = extractRoot;
            psi.UseShellExecute = false;
            Process child = Process.Start(psi);
            child.WaitForExit();
            return child.ExitCode;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("Setup failed: " + ex.Message);
            return 1;
        }
        finally
        {
            try
            {
                if (Directory.Exists(extractRoot))
                {
                    Directory.Delete(extractRoot, true);
                }
            }
            catch
            {
                // Best effort cleanup. Installation already reports its own result.
            }
        }
    }

    private static string QuoteArgument(string arg)
    {
        if (arg == null)
        {
            return "\"\"";
        }
        return "\"" + arg.Replace("\"", "\\\"") + "\"";
    }
}
'@

    Set-Content -Path $sourcePath -Value $source -Encoding ASCII

    if (Test-Path $outputFullPath) {
        Remove-Item -LiteralPath $outputFullPath -Force
    }

    $compileArgs = @(
        "/nologo",
        "/target:exe",
        "/platform:anycpu",
        "/out:$outputFullPath",
        "/resource:$([System.IO.Path]::GetFullPath($ZipPath)),matter-core-windows-x64.zip",
        "/resource:$([System.IO.Path]::GetFullPath($ChecksumJsonPath)),release-checksums.json",
        "/resource:$([System.IO.Path]::GetFullPath($Sha256Path)),SHA256SUMS.txt",
        "/resource:$([System.IO.Path]::GetFullPath($InstallerPath)),install-release-zip.ps1",
        "/resource:$([System.IO.Path]::GetFullPath($LauncherPath)),install-matter-beta.cmd",
        $sourcePath
    )

    & $csc @compileArgs
    if ($LASTEXITCODE -ne 0) {
        throw "csc failed with exit code $LASTEXITCODE"
    }
    if (-not (Test-Path $outputFullPath -PathType Leaf)) {
        throw "Setup exe was not created: $outputFullPath"
    }

    [ordered]@{
        ok = $true
        output = $outputFullPath
        size_bytes = (Get-Item -LiteralPath $outputFullPath).Length
        sha256 = (Get-FileHash -LiteralPath $outputFullPath -Algorithm SHA256).Hash.ToLowerInvariant()
        signed = $false
        signing_note = "Unsigned setup exe. Sign with signtool and a real code-signing certificate before production."
    } | ConvertTo-Json -Depth 4
}
finally {
    if (Test-Path $buildDir) {
        Remove-Item -LiteralPath $buildDir -Recurse -Force
    }
}

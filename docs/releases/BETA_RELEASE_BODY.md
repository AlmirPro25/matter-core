# Matter Core 0.1.0-beta Windows x64 Beta

Status: **beta-ready**  
Channel: **beta**  
Production ready: **False**  
Tag: **v0.1.0-beta**

Matter Core beta is a controlled Windows x64 test release for developers who want to try the experimental Matter runtime, CLI, bytecode VM, JSON automation commands, and guarded reflection.

## Download

Download these files into the same folder:

- matter-core-windows-x64.zip
- install-release-zip.ps1
- release-checksums.json
- SHA256SUMS.txt

## Install

Run from PowerShell in the download folder:

~~~powershell
.\matter-core-beta-setup.exe
~~~

Then open a new PowerShell window and run:

~~~powershell
matter run examples\first_run.matter
matter capabilities-json
~~~

## Diagnose

~~~powershell
powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Matter\scripts\diagnose-local-install.ps1"
~~~

## Uninstall

~~~powershell
powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Matter\scripts\uninstall-local.ps1"
~~~

## Artifact Integrity

- Package: matter-core-windows-x64.zip
- Size: 3.71 MB
- SHA-256: 97a227c9d2bcea31ea471418bf9bbfc3dea9afbbea9e627270bc224d4d7b4b45

Installer artifact:

- install-release-zip.ps1 SHA-256: d2965d91f9854e4f1696310b252ce04467d65f59216d0ee3974538a71a293e9d

Checksum artifacts:

- release-checksums.json SHA-256: 50eb23c9723ef07d5899e07680b705ea1b6cf67de3998dc03991e04821052f70
- SHA256SUMS.txt SHA-256: dfef0cf42f4ebd927e572805dcbef1c2b86bcd4829081362644ccb17ca07fc7a

## Feedback

Use the GitHub issue template **Beta feedback**.

Include:

- Windows version.
- PowerShell version.
- Commands run.
- Diagnosis output.
- Any SmartScreen, antivirus, PATH, permission, or unzip warning.

## Beta Limits

This beta does **not** claim production readiness.

Not included yet:

- Signed .msi installer.
- Code signing certificate.
- Auto-update.
- Linux/macOS installers.
- General production support guarantee.

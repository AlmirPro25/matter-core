# Windows Setup Signing

Matter Core can build an unsigned Windows setup executable:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows-setup-exe.ps1
```

The output is:

```text
dist\matter-core-beta-setup.exe
```

## Production Signing Requirement

For a production-looking Windows installer, sign the setup with a real code-signing
certificate. A self-signed certificate is useful only for local testing; it does
not remove SmartScreen reputation warnings for public users.

Required external pieces:

- Windows SDK with `signtool.exe`.
- OV or EV code-signing certificate from a trusted certificate authority.
- Timestamp server URL, for example `http://timestamp.digicert.com`.

## Sign

With a certificate available to Windows certificate store:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\sign-windows-setup.ps1
```

With a `.pfx` file:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\sign-windows-setup.ps1 `
  -CertificatePath .\certs\matter-code-signing.pfx
```

## Verify

```powershell
signtool verify /pa /v .\dist\matter-core-beta-setup.exe
```

## Current Beta Position

The beta setup EXE is functional but unsigned. It is acceptable for tester builds
when documented clearly. Public production distribution should wait for real
code signing.

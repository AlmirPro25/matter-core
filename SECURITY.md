# Security policy

## Supported surfaces

| Surface | Support posture |
|---------|-----------------|
| Matter Core language-only CLI | Intended for local language runtime use; still pre-1.0 |
| Matter Experimental | Development / research; **not a sandbox** |
| Polyglot bridges, agent UI, network backends | Optional; off in Core default |

## Reporting issues

If you find a vulnerability in Matter Core (especially RCE, sandbox escapes, or secret leakage):

1. Prefer private disclosure to the repository owner.  
2. Do not open a public issue with exploit details until a fix is available.  
3. Include: version/tag, OS, steps to reproduce, impact.

## Hardening already applied (Core 0.1.0 baseline)

- Illegal source tokens rejected; source size/token/depth limits  
- MBC1 validated before run (magic, bounds, jumps, constants)  
- VM limits: stack, call depth, instruction budget, etc.  
- Default binary does not expose shell/PowerShell, agent-ui, net, or package-install commands  
- Experimental local commands use allowlist + injection filters + timeout (still **not** OS isolation)

## Known limitations

- External clean-Windows validation of the portable ZIP is **not** complete (`BLOCKED`)  
- Experimental edition can still perform powerful host actions when built and enabled  
- Stdlib file I/O can write files when used (language feature, not shell)  
- `cargo fmt` / clippy warning debt remains in the broader workspace  

## Secrets

Never commit:

- `.env`, API keys, tokens, private keys  
- Local install paths with credentials  
- User-specific secrets under `apps/` or agent memory files  

See `.gitignore`. Pre-push secret audit is documented in release evidence when performed.

## Dependencies

Language-only dependency posture is recorded under `docs/evidence/releases/0.1.0/dependency-audit.json`. Workspace-wide advisories (e.g. optional polyglot crates) may exist without being linked into Core.

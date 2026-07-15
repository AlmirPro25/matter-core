# Permanent gate: versioned matter-lsp.exe JSON-RPC smoke (no temporary host).
# Usage:
#   .\scripts\test-matter-lsp-binary.ps1
#   .\scripts\test-matter-lsp-binary.ps1 -LspPath path\to\matter-lsp.exe
param(
    [string]$LspPath = "",
    [string]$OutDir = ""
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if (-not $OutDir) {
    $OutDir = Join-Path $Root "target\validation\lsp_delivery_v1"
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Resolve-Lsp([string]$Explicit) {
    if ($Explicit) {
        if (-not (Test-Path -LiteralPath $Explicit)) { throw "LSP not found: $Explicit" }
        return (Resolve-Path -LiteralPath $Explicit).Path
    }
    foreach ($c in @(
        (Join-Path $Root "target\release\matter-lsp.exe"),
        (Join-Path $Root "target\x86_64-pc-windows-gnu\release\matter-lsp.exe")
    )) {
        if (Test-Path -LiteralPath $c) { return (Resolve-Path -LiteralPath $c).Path }
    }
    throw "matter-lsp.exe not found. Build: cargo build -p matter-lsp --release --bin matter-lsp"
}

$lsp = Resolve-Lsp $LspPath
$py = Get-Command python -ErrorAction SilentlyContinue
if (-not $py) {
    $pyPath = "C:\Program Files\Python312\python.exe"
    if (-not (Test-Path $pyPath)) { throw "python required for JSON-RPC smoke" }
} else {
    $pyPath = $py.Source
}

$smokePy = Join-Path $OutDir "jsonrpc_smoke_binary.py"
@'
import json, os, re, subprocess, sys, time

host = sys.argv[1]
out_path = sys.argv[2]
p = subprocess.Popen([host], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0)

def send(obj):
    body = json.dumps(obj).encode("utf-8")
    p.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii") + body)
    p.stdin.flush()

def read_message(timeout=8.0):
    deadline = time.time() + timeout
    buf = b""
    while time.time() < deadline:
        while b"\r\n\r\n" not in buf and time.time() < deadline:
            ch = p.stdout.read(1)
            if not ch:
                time.sleep(0.01)
                continue
            buf += ch
        if b"\r\n\r\n" not in buf:
            return None
        header, rest = buf.split(b"\r\n\r\n", 1)
        m = re.search(br"Content-Length:\s*(\d+)", header, re.I)
        if not m:
            buf = rest
            continue
        n = int(m.group(1))
        body = rest
        while len(body) < n and time.time() < deadline:
            chunk = p.stdout.read(n - len(body))
            if not chunk:
                time.sleep(0.01)
                continue
            body += chunk
        if len(body) < n:
            return None
        msg = body[:n]
        buf = body[n:]
        return json.loads(msg.decode("utf-8"))
    return None

def read_until(pred, timeout=8.0, max_msgs=20):
    msgs = []
    deadline = time.time() + timeout
    while time.time() < deadline and len(msgs) < max_msgs:
        m = read_message(max(0.1, deadline - time.time()))
        if m is None:
            break
        msgs.append(m)
        if pred(m):
            break
    return msgs

results = {}
send({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":os.getpid(),"rootUri":None,"capabilities":{},"clientInfo":{"name":"delivery-smoke","version":"1"}}})
msgs = read_until(lambda m: m.get("id")==1, 8)
init = next((m for m in msgs if m.get("id")==1), None)
results["initialize"] = bool(init and "result" in init)
results["server_name"] = (init or {}).get("result",{}).get("serverInfo",{}).get("name")
send({"jsonrpc":"2.0","method":"initialized","params":{}})
time.sleep(0.15)

def open_doc(uri, text, label):
    send({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"languageId":"matter","version":1,"text":text}}})
    diags=[]
    deadline=time.time()+5
    while time.time()<deadline:
        m=read_message(1.0)
        if m is None: break
        if m.get("method")=="textDocument/publishDiagnostics":
            diags=m.get("params",{}).get("diagnostics") or []
            break
    return {"label":label,"diag_count":len(diags),"messages":" | ".join(d.get("message","") for d in diags)}

samples=[]
samples.append(open_doc("file:///delivery/valid.matter","let x = 1\nprint x\n","valid"))
samples.append(open_doc("file:///delivery/import.matter",'import "x.matter"\n',"import"))
samples.append(open_doc("file:///delivery/panic.matter",'panic("x")\n',"panic"))
results["samples"]=samples
results["valid_ok"]=samples[0]["diag_count"]==0
results["import_unsupported"]="import" in samples[1]["messages"].lower() and "not implemented" in samples[1]["messages"].lower()
results["panic_unsupported"]="panic" in samples[2]["messages"].lower()

send({"jsonrpc":"2.0","id":99,"method":"shutdown","params":None})
msgs=read_until(lambda m: m.get("id")==99, 5)
results["shutdown"]=any(m.get("id")==99 for m in msgs)
send({"jsonrpc":"2.0","method":"exit"})
try:
    p.stdin.close()
except Exception:
    pass
try:
    p.wait(timeout=8)
    results["exit_no_hang"]=True
    results["exit_code"]=p.returncode
except subprocess.TimeoutExpired:
    p.kill()
    results["exit_no_hang"]=False
    results["exit_code"]=None

# orphan check: process must be gone
results["process_gone"]= p.poll() is not None
results["ok"]=all([
    results.get("initialize"),
    results.get("valid_ok"),
    results.get("import_unsupported"),
    results.get("panic_unsupported"),
    results.get("shutdown"),
    results.get("exit_no_hang"),
    results.get("process_gone"),
])
with open(out_path,"w",encoding="utf-8") as f:
    json.dump(results,f,indent=2)
print(json.dumps(results,indent=2))
sys.exit(0 if results["ok"] else 1)
'@ | Set-Content -LiteralPath $smokePy -Encoding utf8

$outJson = Join-Path $OutDir "jsonrpc_smoke_versioned_binary.json"
& $pyPath $smokePy $lsp $outJson
$code = $LASTEXITCODE

# DLL inventory (best-effort PE string scan)
$dlls = @()
try {
    $bytes = [IO.File]::ReadAllBytes($lsp)
    $text = [Text.Encoding]::ASCII.GetString($bytes)
    $dlls = [regex]::Matches($text, '(?i)[a-z0-9_\-]+\.dll') | ForEach-Object { $_.Value.ToLowerInvariant() } | Sort-Object -Unique
} catch {}
$meta = [ordered]@{
    lsp_path = $lsp
    sha256 = (Get-FileHash -LiteralPath $lsp -Algorithm SHA256).Hash
    size = (Get-Item -LiteralPath $lsp).Length
    smoke_exit = $code
    dlls_ascii_scan = $dlls
}
$meta | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $OutDir "matter_lsp_binary_meta.json") -Encoding utf8
Write-Host ("matter-lsp binary smoke exit={0} sha={1}" -f $code, $meta.sha256)
exit $code

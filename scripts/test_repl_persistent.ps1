# Test REPL with persistent state
$commands = @"
let x = 10
print x
let y = x + 5
print y
let z = x * y
print z
:vars
:quit
"@

Write-Host "Testing REPL with persistent state..." -ForegroundColor Cyan
Write-Host ""
$commands | .\target\release\matter-cli.exe repl

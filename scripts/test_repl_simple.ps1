# Test REPL with simple commands
$commands = @"
print 42
let x = 10
print x
:quit
"@

$commands | .\target\release\matter-cli.exe repl

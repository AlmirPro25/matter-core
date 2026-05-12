# Matter Language Tour

This guide documents the syntax that works today. It is written for people trying Matter for the first time.

Run the full tour:

```powershell
.\matter-cli.exe run examples\language_tour.matter
```

If you are building from source:

```powershell
cargo run -q -p matter-cli -- run examples\language_tour.matter
```

## Comments

```matter
# This is a line comment.
```

## Values

```matter
let n = 42
let name = "Matter"
let ok = true
let missing = ()
```

## Printing

```matter
print "hello"
print 42
```

## Variables

Create a variable with `let`:

```matter
let counter = 0
```

Update an existing variable with `set`:

```matter
set counter = counter + 1
```

Use `set` for mutation. `let` creates a new binding in the current scope.

## Operators

```matter
print 10 + 5
print 10 - 5
print 10 * 5
print 10 / 5
print 10 % 3
```

Comparisons:

```matter
print 10 == 10
print 10 != 5
print 10 > 5
print 10 >= 10
print 5 < 10
print 5 <= 5
```

## If / Else

```matter
let score = 82

if score >= 90 {
    print "excellent"
} else {
    print "not-excellent"
}
```

## Functions

```matter
fn square(n) {
    return n * n
}

print square(9)
```

Recursion works:

```matter
fn fib(n) {
    if n <= 1 {
        return n
    }

    return fib(n - 1) + fib(n - 2)
}

print fib(8)
```

## Loops

`while`:

```matter
let i = 0
while i < 3 {
    print i
    set i = i + 1
}
```

`loop`, `break`, and `continue`:

```matter
let j = 0
loop {
    set j = j + 1
    if j == 2 {
        continue
    }
    print j
    if j >= 4 {
        break
    }
}
```

## Lists

```matter
let scores = [95, 82, 67]

print scores
print scores[0]

scores.push(100)
print scores.len()
```

## Maps

```matter
let user = {
    "name": "Ana",
    "age": 21
}

print user.name
print user["age"]

set user.active = true
print user.active
```

## Structs

```matter
struct Point {
    x: int,
    y: int
}

let p = Point {
    x: 10,
    y: 20
}

print p.x
print p.y
```

## Events

```matter
on boot {
    print "event boot"
}

spawn boot
```

You can also trigger event handlers from the CLI:

```powershell
.\matter-cli.exe emit examples\events.matter boot
```

## JSON Tooling

Matter CLI has machine-readable commands for tooling and agents:

```powershell
.\matter-cli.exe check-json examples\language_tour.matter
.\matter-cli.exe reflect-json examples\language_tour.matter
.\matter-cli.exe reflexive-guard-json examples\language_tour.matter
```

Use `check-json` before running generated code. Use `reflect-json` and `reflexive-guard-json` before accepting generated or self-mutating code.


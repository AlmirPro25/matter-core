# Matter Grammar Reference V1 (descriptive)

**Not a formal EBNF from a grammar file** — reconstructed from `matter-parser` call graph.  
**Source:** `crates/matter-parser/src/lib.rs`.

## Program

```
Program := { Statement [';'] }
```

Full input must reduce to `Eof` (Phase 2). Empty program OK (`test_empty_source_ok`).

## Statements

```
Statement :=
    LetStmt
  | SetStmt
  | PrintStmt
  | FunctionDef
  | StructDef
  | ImportStmt
  | ExportStmt
  | OnEvent
  | Spawn
  | IfStmt
  | WhileStmt
  | ForStmt
  | LoopStmt
  | BreakStmt
  | ContinueStmt
  | ReturnStmt
  | MatchStmt
  | EnergyProfile   // sugar: energy profile { ... }
  | ExpressionStmt
```

### Forms (descriptive)

```
LetStmt      := "let" Ident [ ":" Type ] "=" Expression
SetStmt      := "set" ( Ident | Ident "[" Expression "]" | Ident "." Ident )
                 ( "=" | "+=" | "-=" | "*=" | "/=" ) Expression
PrintStmt    := "print" Expression
FunctionDef  := "fn" Ident "(" [ Params ] ")" [ "->" Type ] [ effects ] Block
StructDef    := "struct" Ident "{" { Ident ":" Ident } "}"
ImportStmt   := "import" String
              | "import" "{" Names "}" "from" String
              | "import" String "as" Ident
ExportStmt   := "export" "{" Ident { "," Ident } "}"
OnEvent      := "on" Ident Block
Spawn        := "spawn" Ident
IfStmt       := "if" Expression Block [ "else" ( IfStmt | Block ) ]
WhileStmt    := "while" Expression Block
ForStmt      := "for" Ident "in" Expression Block
LoopStmt     := "loop" Block
BreakStmt    := "break"
ContinueStmt := "continue"
ReturnStmt   := "return" Expression
MatchStmt    := "match" Expression "{" { Expression Block } "}"
Block        := "{" { Statement [';()] } "}"
Params       := Param { "," Param }
Param        := Ident [ ":" Type ]
```

Type grammar supports simple names, `T?`, `T!`, `[T]`, `map<K,V>`, unions `A|B`, `fn(...) -> T`, generics `Name<...>` (`parse_type_annotation`).

## Expressions (precedence)

Lowest → highest binding:

1. `or` / `||`  
2. `and` / `&&`  
3. `== != < > <= >=`  
4. `+ -`  
5. `* / %`  
6. unary `not`/`!`, `-`  
7. postfix: call `()`, index `[]`, field `.`, method `.m()`  
8. primary: literals, ident, `null`, `ok()`/`err()`/`some()`/`none`, list `[]`, map `{}`, lambda `fn (...) { }`, parenthesized  

Optional postfix `?` → `TryPropagate` (`Token::QuestionMark` path).

## Lexical

- Comments: `#` line, `//` line (`skip_comment`)  
- Whitespace / newlines (newlines often filtered in token stream)  
- Illegal unknown chars → `Token::Illegal` (rejected by parser)  

## Limits (parser)

| Limit | Default | Source |
|-------|---------|--------|
| Max source bytes | 1 MiB | `MATTER_MAX_SOURCE_BYTES` / `from_source_checked` |
| Max tokens | 250_000 | `MATTER_MAX_TOKENS` |
| Max recursion depth | 64 | `MAX_RECURSION_DEPTH` |

## Status of grammar productions

| Production | Status |
|------------|--------|
| Core statements (let/set/fn/if/while/for/loop/print/return/struct/on/spawn/match) | IMPLEMENTED_AND_TESTED |
| Import/export productions | PARTIAL (parse only / compile no-op) |
| Type productions | PARTIAL (parse only) |
| Lambda | IMPLEMENTED_UNTESTED |
| `panic` as production | NOT_IMPLEMENTED (token only) |

## Tests anchoring grammar

| Test | File |
|------|------|
| let + type | `matter-parser` `test_parse_let_with_type` |
| fn + types | `test_parse_function_with_types` |
| float | `test_parse_float` |
| logical ops | `test_parse_logical_ops`, `test_parse_word_logical_ops` |
| else if | `test_parse_else_if` |
| illegal / trailing | `test_reject_illegal_character`, `test_reject_garbage_suffix` |
| limits | `test_from_source_checked_respects_source_limit`, `test_deep_nesting_hits_recursion_limit` |
| valid core | `test_valid_core_program_still_parses` |

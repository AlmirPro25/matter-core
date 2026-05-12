# Documented Examples

Examples of Matter code with documentation comments.

## Files

### math_utils.matter
Mathematical utility functions with full documentation.

**Functions:**
- `soma(a, b)` - Add two numbers
- `multiplica(a, b)` - Multiply two numbers
- `fatorial(n)` - Calculate factorial
- `potencia(base, exp)` - Calculate power
- `eh_par(n)` - Check if number is even

## Documentation Format

Matter uses `##` for documentation comments:

```matter
## Function description
##
## Parâmetros:
##   param1 - Description
##   param2 - Description
##
## Retorna:
##   Return value description
##
## Exemplo:
##   let result = function(arg1, arg2)
##   print result
fn function(param1, param2) {
    # Implementation
}
```

## Generating Documentation

### Generate Markdown
```bash
matter-cli docs generate examples/documented/math_utils.matter
```

### Generate HTML
```bash
matter-cli docs generate examples/documented/math_utils.matter --format html
```

### Generate for entire directory
```bash
matter-cli docs generate examples/documented/ --output docs/
```

### Serve documentation locally
```bash
matter-cli docs serve
# Open http://localhost:8080
```

## Documentation Sections

### Description
Main description of the function or module.

### Parâmetros (Parameters)
List of parameters with descriptions:
```matter
## Parâmetros:
##   name - Description
```

### Retorna (Returns)
Description of return value:
```matter
## Retorna:
##   Description of return value
```

### Exemplo (Example)
Code examples:
```matter
## Exemplo:
##   let x = function(10)
##   print x
```

## Best Practices

1. **Always document public functions**
   - Include description
   - List all parameters
   - Describe return value
   - Provide example

2. **Keep descriptions concise**
   - One line for summary
   - Additional lines for details

3. **Provide realistic examples**
   - Show actual usage
   - Include expected output
   - Cover common cases

4. **Update docs with code**
   - Keep documentation in sync
   - Update examples when API changes
   - Remove outdated information

## Output Formats

### Markdown (.md)
- Easy to read
- Version control friendly
- Can be converted to other formats

### HTML (.html)
- Professional appearance
- Syntax highlighting
- Interactive navigation
- Search functionality (future)

### JSON (.json)
- Machine-readable
- API integration
- Tool consumption

## Integration

### VS Code
Documentation appears in hover tooltips and autocomplete.

### LSP
Language server provides documentation on hover.

### REPL
`:help function_name` shows documentation.

## Examples

### Simple Function
```matter
## Doubles a number
##
## Parâmetros:
##   n - Number to double
##
## Retorna:
##   n * 2
##
## Exemplo:
##   print dobro(21)  # 42
fn dobro(n) {
    return n * 2
}
```

### Complex Function
```matter
## Filters a list based on a condition
##
## Applies a filter function to each element
## and returns a new list with matching elements.
##
## Parâmetros:
##   lista - List to filter
##   condicao - Filter function (returns bool)
##
## Retorna:
##   New list with filtered elements
##
## Exemplo:
##   let numbers = [1, 2, 3, 4, 5]
##   let pares = filtrar(numbers, eh_par)
##   print pares  # [2, 4]
fn filtrar(lista, condicao) {
    let resultado = []
    for item in lista {
        if condicao(item) {
            list.push(resultado, item)
        }
    }
    return resultado
}
```

## Contributing

When adding new examples:
1. Include full documentation
2. Follow format guidelines
3. Provide realistic examples
4. Test generated documentation

---

**Last Updated:** May 9, 2026  
**Version:** v0.7.0

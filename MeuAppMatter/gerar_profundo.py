import sys

depth = 5000

with open(sys.argv[1], "w") as f:
    f.write('print "Iniciando AST profundo..."\n')
    for i in range(depth):
        f.write(' ' * i + 'if true {\n')
    
    f.write(' ' * depth + 'print "ATINGIU O FUNDO DO POCO!"\n')
    
    for i in range(depth - 1, -1, -1):
        f.write(' ' * i + '}\n')
    
    f.write('print "Sobreviveu ao parser!"\n')

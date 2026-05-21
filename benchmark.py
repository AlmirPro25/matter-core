import time

def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)

print("Iniciando Teste de Furia (Python)...")
start_time = time.time()

print("1. Fibonacci(35)...")
start_fib = fib(35)
print("Resultado:", start_fib)

print("2. Loop pesado com manipulacao de Dicionario/Map na Heap")
d = {"contador": 0}
i = 0
while i < 10000:
    d["contador"] = d["contador"] + 1
    i = i + 1

print("Contador processado:", d["contador"])

end_time = time.time()
print(f"Tempo total Python: {end_time - start_time:.4f} segundos")

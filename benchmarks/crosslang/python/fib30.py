import time

def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

s = time.perf_counter()
result = fib(30)
e = time.perf_counter()
print(result)
print((e - s) * 1000.0)

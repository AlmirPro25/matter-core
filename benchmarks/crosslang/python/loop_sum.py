import time
s = time.perf_counter()
total = 0
for i in range(1, 1000001):
    total += i
e = time.perf_counter()
print(total)
print((e - s) * 1000.0)

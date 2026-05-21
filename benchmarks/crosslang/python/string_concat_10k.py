import time
s = time.perf_counter()
out = ""
for _ in range(10000):
    out += "a"
e = time.perf_counter()
print(len(out))
print((e - s) * 1000.0)

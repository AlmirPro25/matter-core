import time
s = time.perf_counter()
nums = list(range(10000, 0, -1))
nums.sort()
total = sum(nums)
e = time.perf_counter()
print(total)
print((e - s) * 1000.0)

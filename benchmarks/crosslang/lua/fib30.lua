local function fib(n)
  if n <= 1 then return n end
  return fib(n - 1) + fib(n - 2)
end
local s = os.clock()
local result = fib(30)
local e = os.clock()
print(result)
print((e - s) * 1000.0)

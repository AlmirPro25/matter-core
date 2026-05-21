local s = os.clock()
local total = 0
for i = 1, 1000000 do
  total = total + i
end
local e = os.clock()
print(total)
print((e - s) * 1000.0)

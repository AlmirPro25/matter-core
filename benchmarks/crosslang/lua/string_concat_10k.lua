local s = os.clock()
local out = ""
for i = 1, 10000 do
  out = out .. "a"
end
local e = os.clock()
print(#out)
print((e - s) * 1000.0)

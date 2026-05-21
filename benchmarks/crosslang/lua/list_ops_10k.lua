local s = os.clock()
local nums = {}
for i = 10000, 1, -1 do
  table.insert(nums, i)
end
table.sort(nums)
local total = 0
for i = 1, #nums do
  total = total + nums[i]
end
local e = os.clock()
print(total)
print((e - s) * 1000.0)

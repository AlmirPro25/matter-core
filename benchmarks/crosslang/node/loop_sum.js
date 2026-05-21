const { performance } = require("perf_hooks");
const s = performance.now();
let total = 0;
for (let i = 1; i <= 1000000; i++) total += i;
const e = performance.now();
console.log(total);
console.log(e - s);

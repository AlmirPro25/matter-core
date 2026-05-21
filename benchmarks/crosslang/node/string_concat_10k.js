const { performance } = require("perf_hooks");
const s = performance.now();
let out = "";
for (let i = 0; i < 10000; i++) out += "a";
const e = performance.now();
console.log(out.length);
console.log(e - s);

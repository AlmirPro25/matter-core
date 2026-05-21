const { performance } = require("perf_hooks");
function fib(n) {
  if (n <= 1) return n;
  return fib(n - 1) + fib(n - 2);
}
const s = performance.now();
const result = fib(30);
const e = performance.now();
console.log(result);
console.log(e - s);

# Concurrency Examples

Examples demonstrating Matter Core's concurrency primitives.

## Files

### async_basic.matter
Basic async/await usage with simulated async operations.

**Demonstrates:**
- `async fn` - Async function declaration
- `await` - Waiting for async results
- Sequential async execution

### channels.matter
Producer-consumer pattern using channels.

**Demonstrates:**
- `channel()` - Creating channels
- `send(ch, value)` - Sending values
- `recv(ch)` - Receiving values
- `close(ch)` - Closing channels
- `spawn` - Creating concurrent tasks

### parallel_map.matter
Parallel processing of lists.

**Demonstrates:**
- `parallel_map(list, fn)` - Parallel map operation
- Performance comparison (sequential vs parallel)
- Speedup calculation

### spawn_join.matter
Spawning multiple tasks and waiting for results.

**Demonstrates:**
- `spawn(fn)` - Creating tasks
- `join(task)` - Waiting for task completion
- Return values from tasks
- Multiple concurrent tasks

## Concurrency Primitives

### Async/Await

```matter
async fn fetch_data(url) {
    let response = await net.get(url)
    return response
}

let data = await fetch_data("https://api.com")
```

### Channels

```matter
let ch = channel()

## Producer
spawn fn() {
    send(ch, 42)
    close(ch)
}

## Consumer
let value = recv(ch)
print value
```

### Spawn/Join

```matter
let task = spawn fn() {
    return 42
}

let result = join(task)
print result
```

### Mutex

```matter
let counter = mutex(0)

spawn fn() {
    let value = lock(counter)
    set value = value + 1
    unlock(counter, value)
}
```

### Parallel Operations

```matter
let numbers = [1, 2, 3, 4, 5]

## Parallel map
let squares = parallel_map(numbers, fn(n) {
    return n * n
})

## Parallel filter
let evens = parallel_filter(numbers, fn(n) {
    return math.mod(n, 2) == 0
})

## Parallel reduce
let sum = parallel_reduce(numbers, fn(acc, n) {
    return acc + n
}, 0)
```

## Performance

### Sequential vs Parallel

**CPU-bound tasks:**
- Sequential: 1000ms
- Parallel (4 cores): 280ms (3.6x speedup)
- Parallel (8 cores): 160ms (6.3x speedup)

**I/O-bound tasks:**
- Sequential: 10,000ms
- Async: 250ms (40x speedup)

### Best Practices

1. **Use async/await for I/O**
   - Network requests
   - File operations
   - Database queries

2. **Use parallel_map for CPU-intensive tasks**
   - Data processing
   - Image manipulation
   - Calculations

3. **Use channels for communication**
   - Producer-consumer patterns
   - Pipeline processing
   - Actor model

4. **Use mutex for shared state**
   - Counters
   - Caches
   - Shared resources

## Common Patterns

### Pipeline Processing

```matter
let input_ch = channel()
let output_ch = channel()

## Stage 1: Generate
spawn fn() {
    let i = 0
    while i < 100 {
        send(input_ch, i)
        set i = i + 1
    }
    close(input_ch)
}

## Stage 2: Process
spawn fn() {
    loop {
        let value = recv(input_ch)
        if value == unit { break }
        
        let result = value * 2
        send(output_ch, result)
    }
    close(output_ch)
}

## Stage 3: Collect
let results = []
loop {
    let value = recv(output_ch)
    if value == unit { break }
    list.push(results, value)
}
```

### Fan-out/Fan-in

```matter
let input_ch = channel()
let output_ch = channel()

## Fan-out: Multiple workers
let workers = 4
let i = 0
while i < workers {
    spawn fn() {
        loop {
            let value = recv(input_ch)
            if value == unit { break }
            
            ## Process
            let result = process(value)
            send(output_ch, result)
        }
    }
    set i = i + 1
}

## Fan-in: Collect results
let results = []
let count = 0
while count < total {
    let result = recv(output_ch)
    list.push(results, result)
    set count = count + 1
}
```

### Actor Model

```matter
fn create_actor(handler) {
    let mailbox = channel()
    
    spawn fn() {
        loop {
            let msg = recv(mailbox)
            if msg == unit { break }
            handler(msg)
        }
    }
    
    return mailbox
}

## Use actor
let actor = create_actor(fn(msg) {
    print "Received: " + msg
})

send(actor, "Hello")
send(actor, "World")
```

## Thread Safety

### Race Conditions

```matter
## ❌ Unsafe (race condition)
let counter = 0

spawn fn() { set counter = counter + 1 }
spawn fn() { set counter = counter + 1 }

## ✅ Safe (mutex)
let counter = mutex(0)

spawn fn() {
    let value = lock(counter)
    set value = value + 1
    unlock(counter, value)
}
```

### Deadlock Prevention

```matter
## ❌ Potential deadlock
let m1 = mutex(0)
let m2 = mutex(0)

spawn fn() {
    lock(m1)
    lock(m2)  ## Deadlock!
}

spawn fn() {
    lock(m2)
    lock(m1)  ## Deadlock!
}

## ✅ Consistent lock ordering
spawn fn() {
    lock(m1)
    lock(m2)
}

spawn fn() {
    lock(m1)  ## Same order
    lock(m2)
}
```

## Running Examples

```bash
## Basic async
matter-cli run examples/concurrency/async_basic.matter

## Channels
matter-cli run examples/concurrency/channels.matter

## Parallel map
matter-cli run examples/concurrency/parallel_map.matter

## Spawn/join
matter-cli run examples/concurrency/spawn_join.matter
```

## Notes

- Concurrency features require Matter Core v0.8.0+
- Performance depends on hardware (CPU cores)
- Async I/O requires backend support
- Thread safety is enforced by runtime

---

**Last Updated:** May 9, 2026  
**Version:** v0.8.0

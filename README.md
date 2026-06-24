# spin-lock

A small experiment comparing a `std::sync::Mutex`-protected buffer against a hand-rolled
`SpinLock` buffer. Each workload spawns 100 threads that each bump one slot of a shared
`[u64; 100]`.

## Benchmarks

Run with [criterion](https://github.com/bheisler/criterion.rs):

```sh
cargo bench
```

| benchmark          | low       | median    | high      |
| ------------------ | --------- | --------- | --------- |
| `mutexed_buffer`   | 892.11 µs | 897.17 µs | 902.87 µs |
| `spin_lock_buffer` | 881.93 µs | 888.03 µs | 894.61 µs |

## Reading the results

The two are within ~1% of each other (~890 µs). That near-tie is the point: each iteration
spawns and joins 100 OS threads, and that lifecycle cost dominates the measurement. Actual
lock behavior is noise next to it, so this bench does **not** meaningfully compare the locks
— it mostly measures thread creation.

To actually contrast the locks, the workload would need to remove thread overhead from the
hot path — e.g. a fixed pool of long-lived threads hammering the lock in a loop, where the
spin lock's busy-waiting and the mutex's park/wake trade-offs would actually show up.

# Mysterious Cache

About the quickest and dirtiest implementation of an LRU Cache in Rust.

A Least Recently Used, or LRU, Cache, is a data structure that can be thought of
as a hash map with a maximum size that evicts the oldest element whenever it is
at capacity and a new element is inserted. They're commonly used to keep the
hottest parts of large data sets in memory while letting the colder or less
frequently used data getting frequently evicted.

```rust
let mut cache: LruCache<usize, String> = LruCache::with_capacity(5);

cache.insert(0, "Put".to_owned());
cache.insert(1, "large".to_owned());
cache.insert(2, "things".to_owned());
cache.insert(3, "in".to_owned());
cache.insert(4, "memory".to_owned());
cache.insert(5, "but not too many".to_owned());

assert_eq!(None, cache.get(&0));
```

Internally uses no unsafety, no Rc to create the linked list that backs the
eviction queue. Whether avoiding these things was wise or not is a matter of
debate. The linked list for the eviction queue is stored on a Vec, so this isn't
going to do interesting things with heap fragmentation outside of what HashMap
will already do. It should also help with data locality and processor memory
prefetching, but I haven't done any testing to see if any of this is validated.

In addition there are two other useful caches provided, an Expiring Cache and a
Shared Cache.

ExpiringCache behaves the same way as LruCache, except that on construction it
is assigned a *timeout*, which is a duration that elements in the cache must not
be older than in order to be returned.

```rust
let mut cache: ExpiringCache<u64, u64> =
    ExpiringCache::with_capacity_and_timeout(1, Duration::from_secs(30));
cache.insert(1, 1);
assert_eq!(Some(1), cache.get(&1));
sleep(Duration::from_secs(31));
assert_eq!(None, cache.get(&1));
```

SharedCache can wrap either LruCache or ExpiringCache and provides a
Send + Sync container for them, making it slightly easier to use in situations
where it has to be shared across thread boundaries.

```rust
let cache: SharedCache<LruCache<usize, usize>, usize, usize> =
    SharedCache::with_cache(LruCache::with_capacity(1));
cache.insert(1, 1);

let thread_cache = cache.clone();
thread::spawn(move || {
    thread_cache.get(&1)
})
.join();
```

## Using

In my opinion this is not mature enough to be put on crates.io. If you'd like to
put some miles into this software, please use it directly from github.

```toml
[dependencies]
mysterious_cache = { git = "https://github.com/mysteriouspants/mysterious_cache" }
```

## License

I want you to be able to use this software regardless of who you may be, what
you are working on, or the environment in which you are working on it - I hope
you'll use it for good and not evil! To this end, mysterious_cache is licensed
under the [2-clause BSD license][2cbsd], with other licenses available by
request. Happy coding!

[2cbsd]: https://opensource.org/licenses/BSD-2-Clause

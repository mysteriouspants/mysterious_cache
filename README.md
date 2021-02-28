# Mysterious Cache

About the quickest and dirtiest implementation of an LRU cache in Rust.

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

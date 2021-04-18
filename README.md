# const-regex

<!-- cargo-sync-readme start -->

Proc macro to match regexes in const fns. The regex must be a string literal, but the bytes
matched can be any value.

The macro expects an `&[u8]`, but you can easily use `str::as_bytes`.

```rust
const fn this_crate(bytes: &[u8]) -> bool {
    const_regex::match_regex!("^(meta-)*regex matching", bytes)
}

assert!(this_crate(b"meta-meta-regex matching"));
assert!(!this_crate(b"a good idea"));
```

<!-- cargo-sync-readme end -->

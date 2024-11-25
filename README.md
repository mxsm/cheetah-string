# 🐆 Cheetah String

**A lightweight, high-performance string manipulation library optimized for speed-sensitive applications.**

Cheetah String is designed to provide fast and efficient string operations with minimal resource usage, ideal for performance-critical and memory-constrained environments. It offers a range of easy-to-use, high-speed string handling functions.

## Features

- **Highly optimized**: Performance-tuned across platforms to support low-latency needs.
- **Comprehensive API**: Includes common string operations (e.g., split, concatenate, transform, format, etc.).
- **Easy to integrate**: Simple, intuitive design to minimize the learning curve.
- **Cross-platform**: Compatible with major operating systems and development environments.

## How to use

To use **`Cheetah-String`**, first add this to your `Cargo.toml`:

```toml
[dependencies]
cheetah-string = "0.1.0"
```

### Bytes support

**Bytes** support is optional and disabled by default. To enable use the feature `bytes`.

```toml
[dependencies]
cheetah-string = { version = "1", features = ["bytes"] }
```

### Serde support

**serde** support is optional and disabled by default. To enable use the feature `serde`.

```toml
[dependencies]
cheetah-string = { version = "1", features = ["serde"] }
```

##  Projects used

- [**Rocketmq-rust**](https://github.com/mxsm/rocketmq-rust)

## Contributing

We welcome issues and pull requests to help build a more efficient string manipulation library together!

## License

**cheetah-string** is licensed under the [Apache License 2.0](https://github.com/mxsm/cheetah-string/blob/main/LICENSE) and [MIT license](https://github.com/mxsm/cheetah-string/blob/main/LICENSE-MIT)
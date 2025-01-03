# Rich Logger
[![crates.io](https://img.shields.io/crates/v/rich-logger.svg)](https://crates.io/crates/rich-logger)

A beautiful and performant implementation for log inspired by pythons [`rich`](https://pypi.org/project/rich/) package

## Usage

```console
$ cargo add log rich-logger
```

```rs
use log::{info, LevelFilter};

fn main() {
    rich_logger::init(LevelFilter::Debug).expect("Failed to initialize logger!");
    info!("Hello, World!");
    
    // ...
}
```

# Features
## async
 - ensures each log is handled atomically and efficiently by passing the message to a background thread. Do not use when immediate output is required.
    
## json
 - allows json to be logged in a pretty printed format

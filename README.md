# Inline tweak

![](https://i.imgur.com/DZrg910.gif)

[![Crates.io](https://img.shields.io/crates/v/inline_tweak.svg)](https://crates.io/crates/inline_tweak)

Inline tweaks is based on [this blogpost](http://blog.tuxedolabs.com/2018/03/13/hot-reloading-hardcoded-parameters.html)
by tuxedo labs.  

Tweak any number literal directly from your code, changes to the source appear while running the program.  
It works by parsing the file when a change occurs.  

The library is minimal, only requiring the lazy_static dependency to hold modified values.  
In release mode, the tweaking code is disabled and compiled away.  

## Usage

```rust
use inline_tweak::*;

fn main() {
    loop {
        println!("{}", tweak!(3.14)); // Try changing the value while the application is running
    }
}
```

`inline_tweak` also provides a `watch!()` macro that sleeps until the file is modified, akin to a breakpoint:
```rust
use inline_tweak::*;

fn main() {
    loop {
        println!("{}", tweak!(3.14));
        watch!(); // The thread will sleep here until anything in the file changes
    }
}
```

## Installation

Simply add this line to your Cargo.toml

```toml
inline_tweak = "1.0.1"
```

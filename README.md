# VAWK (Visual AWK)

This projects provides a graphical environment for splitting & viewing shell command output.  It aims to be a more intuitive way to view subsets of stdout, where previously you would have to research an `awk` command.

https://user-images.githubusercontent.com/4751760/127952090-75102fac-5f3f-4fd7-a308-d63596e14715.mov

## Usage

VAWK is run as a single standalone binary.  HTML/CSS/JS is packaged and included in the binary.  To build from source, run

```
cargo build
```

You will need

- [Rust](https://www.rust-lang.org/)
- [Node](https://nodejs.org/en/)
- [protoc](https://grpc.io/docs/protoc-installation/)

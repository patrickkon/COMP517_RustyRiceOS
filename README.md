# RustyRiceOS

This repository contains the source code for our mini-kernel for the course COMP517 in Rice University.

**Please build from the master branch.**

## Building

This project requires a nightly version of Rust because it uses some unstable features. At least nightly _2020-07-15_ is required for building. You might need to run `rustup update nightly --force` to update to the latest nightly even if some components such as `rustfmt` are missing it.

You can build the project by running:

```
cargo build
```

To create a bootable disk image from the compiled kernel, you need to install the [`bootimage`] tool:

[`bootimage`]: https://github.com/rust-osdev/bootimage

```
cargo install bootimage
```

After installing, you can create the bootable disk image by running:

```
cargo bootimage
```

This creates a bootable disk image in the `target/x86_64-rusty_rice_os/debug` directory.

Please file an issue if you have any problems.

## Running

You can run the disk image in [QEMU] through:

[QEMU]: https://www.qemu.org/

```
cargo run
```

[QEMU] and the [`bootimage`] tool need to be installed for this.

You can also write the image to an USB stick for booting it on a real machine. On Linux, the command for this is:

```
dd if=target/x86_64-rusty_rice_os/debug/bootimage-rusty_rice_os.bin of=/dev/sdX && sync
```

Where `sdX` is the device name of your USB stick. **Be careful** to choose the correct device name, because everything on that device is overwritten.

## Testing

To run all the unit and integration tests, execute `cargo test`.

## Commands available in our kernel
The commands are made available through our REPL program, which is made available when you launch our kernel (instructions above).
```
# Command to exit the kernel
$ exit 

# Command to execute our built-in heap allocation test:
$ alloc-dealloc-test
```

## Testing our in-house heap allocator's alloc-dealloc speed
```
# Simple Test that uses /tests/heap_allocation_simple.rs
$ cd tests
$ sh tests.sh

X milliseconds have elapsed.

# Comprehensive Test that uses /tests/heap_allocation.rs:
# How? In tests.sh, change all occurences of heap_allocation_simple to heap_allocation, and then re-execute the above commands used in Simple Test
```

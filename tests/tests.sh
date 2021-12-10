#!/usr/bin/env bash

# SECTION: Build tests first.
# cargo build --test heap_allocation_simple --profile test
# cargo test --test heap_allocation_simple --release --no-run
# Not using no-run because bootloader crate is only compiled in this case apparently. 
cargo test --test heap_allocation_simple --release 2>/dev/null
echo " --------------------------- FINISHED BUILDING ------------------------------"
# SECONDS=0
ts=$(date +%s%N)
# SECTION: Testing with the release profile can give us speedups:
# https://doc.rust-lang.org/cargo/commands/cargo-test.html
# https://nnethercote.github.io/perf-book/build-configuration.html
cargo test --test heap_allocation_simple --release

# duration=$SECONDS
echo " --------------------------- FINISHED TESTING ------------------------------"
echo "$((($(date +%s%N) - $ts)/1000000)) milliseconds elapsed."
# echo "$(($duration / 60)) minutes and $(($duration % 60)) seconds elapsed."
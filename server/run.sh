#!/bin/sh

diesel migration run
cargo run --release

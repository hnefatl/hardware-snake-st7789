#!/bin/bash

cargo build --release || exit 1

openocd -f openocd.cfg -c "program target/thumbv7em-none-eabihf/release/snake reset exit"

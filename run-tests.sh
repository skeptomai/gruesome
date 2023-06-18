#!/bin/sh

DATE=$(date +"%Y-%m-%dT%H:%M:%S%z")
RUST_LOG=debug cargo test -- --nocapture 2>&1 | tee log-$DATE.txt

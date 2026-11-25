#!/bin/sh
set -eu

limit=200
find . -type d \( -name .git -o -name target \) -prune -o \
    -type f -name '*.rs' -exec sh -c '
        limit=$1
        shift
        for file do
            lines=$(wc -l < "$file")
            if [ "$lines" -gt "$limit" ]; then
                printf "%s: %s lines (limit: %s)\n" "$file" "$lines" "$limit" >&2
                exit 1
            fi
        done
    ' sh "$limit" {} +

printf 'Rust source files comply with the %s-line limit.\n' "$limit"

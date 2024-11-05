#!/bin/bash

$PATH-TO-YOUR-APP # example ~/dev/git_glance/target/release/git_glance

if [[ -f dir_path.txt && -s dir_path.txt ]]; then
    dir_path=$(cat dir_path.txt)

    rm dir_path.txt

    cd "$dir_path" || {
        echo "Directory not found"
    }
fi

clear

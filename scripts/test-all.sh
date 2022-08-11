#!/usr/bin/env bash

set -euo pipefail
IFS=$'\n\t'

if ! [ -x "$(command -v curl)" ] ; then
  >&2 echo "'curl' is required for downloading test data"
  exit 1
fi

cache_dir=sentencepiece/testdata
test_dir=testdata

declare -A models=(
  ["ALBERT_BASE_MODEL"]="https://s3.amazonaws.com/models.huggingface.co/bert/albert-base-v1-spiece.model")

if [ ! -d "$cache_dir" ]; then
  mkdir -p "$cache_dir"
fi

for var in "${!models[@]}"; do
  url="${models[$var]}"
  bn="$(basename "${url}")"
  data="${cache_dir}/${bn}"

  if [ ! -e "${data}" ]; then
    curl -fo "${data}" "${url}"
  fi

  declare -x "${var}"="${test_dir}/${bn}"
done

cargo test --features albert-tests


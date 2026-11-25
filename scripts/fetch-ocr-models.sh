#!/usr/bin/env bash
set -euo pipefail

destination=${1:?Usage: fetch-ocr-models.sh <destination-directory>}
mkdir -p "$destination"

curl --fail --location --silent --show-error \
  https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.onnx \
  --output "$destination/text-detection.onnx"
curl --fail --location --silent --show-error \
  https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.onnx \
  --output "$destination/text-recognition.onnx"

printf '%s  %s\n' \
  a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe \
  "$destination/text-detection.onnx" \
  86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5 \
  "$destination/text-recognition.onnx" | sha256sum --check --status

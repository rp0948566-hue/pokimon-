#!/bin/bash
# Retries the official DDR Google Drive download every 20 min (quota blocks lift on their own, up to ~24 h).
# No quota bypass: plain re-run of download_datasets.py. Stops when all 10 parts exist or after 72 tries.
cd "$(dirname "$0")/.."
for i in $(seq 1 72); do
  n=$(ls data/downloads/ddr/DDR-dataset.zip.0* 2>/dev/null | grep -vc '\.part$')
  [ "$n" -ge 10 ] && { echo "$(date -Is) all 10 parts present"; exit 0; }
  echo "$(date -Is) attempt $i (parts present: $n)"
  .venv/bin/python scripts/download_datasets.py --dataset ddr --download --i-accepted-terms 2>&1 | grep -v "^Processing\|^Retrieving" | tail -3 | cut -c1-200
  sleep 1200
done

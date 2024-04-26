#!/bin/bash
cd contracts/
for contract in `ls ./`; do
  echo "generating schema for ${contract}"
  cd ${contract}/
  cargo run schema
  rm -rf ./schema/raw
  cd ../
done

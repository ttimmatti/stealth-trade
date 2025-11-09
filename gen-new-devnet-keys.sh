#!/bin/bash

keys=(
  devnet-mint
  devnet-group
  devnet-otherGroup
)

for key in "${keys[@]}"; do
  out="tmp/$key.json"
  echo "Generating key $key -> $out"
  solana-keygen new --force --no-bip39-passphrase --outfile "$out"
done

#!/bin/bash

keys=(
  devnet-user
  devnet-otherUser
  devnet-mint-a
  devnet-mint-b
  devnet-group
  devnet-otherGroup
)

for key in "${keys[@]}"; do
  out="tmp/$key.json"
  echo "Generating key $key -> $out"
  solana-keygen new --force --no-bip39-passphrase --outfile "$out"
done

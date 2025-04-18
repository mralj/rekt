#!/bin/bash

GREEN='\033[0;32m'
NC='\033[0m' # No Color
HOME="/home/ubuntu"
REKT_RELEASE="$HOME/rekt/target/release/"
cd $HOME

echo "======== STARTED RUST SETUP ========="
curl https://sh.rustup.rs -sSf | sh -s -- -y

source $HOME/.profile
source $HOME/.bash_profile
source $HOME/.bashrc
source $HOME/.cargo/env
source /etc/profile

cargo version

cd $HOME

echo "====== BUILDING THE CODE ==========="
echo "Please wait..."
cd $HOME/rekt/
cargo build --release

echo "[]" > $REKT_RELEASE/tokens_to_buy.json
cd $HOME

sudo chmod -R 777 $HOME/rekt
echo -e "${GREEN} DONE :D ${NC}"


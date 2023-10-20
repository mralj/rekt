#!/bin/bash

GREEN='\033[0;32m'
NC='\033[0m' # No Color
HOME="/home/ec2-user"
REKT_RELEASE="$HOME/rekt/target/release/"
cd $HOME

# aliases="alias ll='ls -lrth'\nalias gi='git'\nalias gir='git rest --hard'\nalias tmxn='tmux new-session -A -s node'\nalias mge='make geth && mv ./build/bin/geth ../node'\nalias gmge='git pull && mge'"


# Append the lines to the .bashrc file
#echo -e "$aliases" >> $HOME/.bashrc
touch .profile

screen_config="defscrollback 10000 \n termcapinfo xterm* ti@:te@ \n"
echo -e "$screen_config" >> $HOME/.screenrc

echo "Installing htop, git etc."
sudo yum update -y &> /dev/null
sudo yum install -y gcc kernel-devel make ncurses-devel &> /dev/null
sudo amazon-linux-extras install epel -y &> /dev/null
sudo yum-config-manager --enable epel &> /dev/null
sudo yum install -y git htop nload &> /dev/null

echo -e "${GREEN} Installed git, htop ... ${NC}"

sleep 4

cd $HOME
source $HOME/.profile
source $HOME/.bash_profile
source $HOME/.bashrc
source $HOME/.screenrc
source /etc/profile


echo "======== STARTED RUST SETUP ========="
curl https://sh.rustup.rs -sSf | sh -s -- -y
source $HOME/.profile
source $HOME/.bash_profile
source /etc/profile
cargo version

sleep 4

cd $HOME

echo "====== BUILDING THE CODE ==========="
echo "Please wait..."

cd $HOME/rekt/
cargo build --release

echo "{ \"tokens\": [] }" > $REKT_RELEASE/tokens_to_buy.json
cd $HOME

echo "====== STARTING THE NODE ==========="
sudo chmod -R 777 $HOME/rekt


echo -e "${GREEN} DONE :D ${NC}"



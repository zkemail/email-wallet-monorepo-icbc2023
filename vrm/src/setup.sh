#!/bin/bash

cp .env ./relayer/.env
echo "EMAIL_DIR=./emails" >> ./relayer/.env
echo "EMAIL_DIR=./emails" >> ./relayer/.env 
echo "APP_PARAM_PATH=./configs/app_params.bin" >> ./relayer/.env
echo "AGG_PARAM_PATH=./configs/agg_params.bin" >> ./relayer/.env
echo "MANIPULATION_DEFS_PATH=./configs/manipulation_defs.json" >> ./relayer/.env
echo "WALLET_ABI_PATH=./configs/EmailWallet.json" >> ./relayer/.env
echo "ERC20_ABI_PATH=./configs/IERC20.json" >> ./relayer/.env
echo "IMAN_ABI_PATH=./configs/IManipulator.json" >> ./relayer/.env
APP_PARAM="./relayer/configs/agg_params.bin"
if [ ! -e $APP_PARAM ]; then
    wget -O $APP_PARAM https://trusted-setup-halo2kzg.s3.eu-central-1.amazonaws.com/perpetual-powers-of-tau-raw-24
fi

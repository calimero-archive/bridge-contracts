#!/bin/bash
set -e

cd contracts

echo "Light client tests start..."
cd light_client
cargo test -- --nocapture
echo "Light client tests done."

echo "Prover tests start..."
echo "-Mock light client build started..."
cd ../mock_light_client
./build.sh
echo "-Mock light client build done."
cd ../prover
./build.sh
cargo test -- --nocapture
echo "Prover tests done."

echo "Types tests start..."
cd ../types
cargo test -- --nocapture
echo "Types tests done."

echo "Connector Permissions tests start..."
cd ../connector_permissions
./build.sh
cargo test -- --nocapture
echo "Connector Permissions tests done."

echo "FT Connector tests start..."
echo "-FT bridge token build and deploy started..."
cd ../ft_bridge_token
./build.sh
./deploy.sh
echo "-FT bridge token build and deploy done."
echo "-Bridge token deployer build started..."
cd ../bridge_token_deployer
./build_ft.sh
./test_ft.sh
echo "-Bridge token deployer build done."
echo "-Mock prover build started..."
cd ../mock_prover
./build.sh
echo "-Mock prover build done."
echo "-Connector permissions build started..."
cd ../connector_permissions
./build.sh
echo "-Connector permissions build done."
cd ../ft_connector
./build.sh
cargo test -- --nocapture
echo "FT Connector tests done."

echo "NFT Connector tests start..."
echo "-NFT bridge token build and deploy started..."
cd ../nft_bridge_token
./build.sh
./deploy.sh
echo "-NFT bridge token build and deploy done."
echo "-Bridge token deployer build started..."
cd ../bridge_token_deployer
./build_nft.sh
./test_nft.sh
echo "-Bridge token deployer build done."
cd ../nft_connector
./build.sh
cargo test -- --nocapture
echo "NFT Connector tests done."

echo "XSC Connector tests start..."
cd ../xsc_connector
./build.sh
cargo test -- --nocapture
echo "XSC Connector tests done."

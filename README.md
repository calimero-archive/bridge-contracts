# Contracts

## Light client

The light client contract will accept block headers that are being relayed to it if the blocks meet certain validation criteria - most notably each valid block header needs to contain at least two thirds valid signatures of the epoch block producers

```
cd contracts/light_client
./build.sh
```

deploy the contract on NEAR Testnet and on Calimero
```
./deploy.sh
./deploy_calimero.sh calimero_shard_name
```

## Prover

What makes the light client contract interesting is the ability to prove that something happened on a specific chain. The prover takes as input the proof data that contain a merkle path to the block where the transaction/receipt originated and a merkle path to the transaction/receipt, also as input the height of the known block to the light client contract needs to be provided, and this block needs to be ahead or on the block of the transaction that we are proving. With all of this the prover can calculate the expected block merkle root and compare it to the one stored in the light client block.

Prerequisite for deploying the prover is that the light_client contract is already deployed
```
cd contracts/prover
./build.sh
./deploy.sh
./deploy_calimero.sh calimero_shard_id
```

## FT connector

The prover enables us to build a set of contracts for transferring assets from one chain to another, or even to make cross shard contract calls. Most notably, Calimero supports transfering Fungible tokens as well as Non Fungible tokens from one chain to another.

Prerequisite for deploying the connectors is that the prover contract on each chain is already deployed
```
cd bridge_token
./build.sh
./deploy.sh
cd ../ft_connector_source
./build.sh
./deploy.sh
cd ../ft_connector_destination
./build.sh
./deploy_calimero.sh calimero_shard_id
```

# High level overview

All contracts in this repo are needed to make the bidirectional bridge from NEAR to Calimero work.

Below is an image of all the components of the bridge. To read more about how these components interact go to [the official documentation](https://docs.calimero.network/).

![08](https://user-images.githubusercontent.com/1136810/197786531-587f9d4a-d6c7-4fa8-8416-28dc381ba88d.jpg)


# Light client

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

# Prover

What makes the light client contract interesting is the ability to prove that something happened on a specific chain. The prover takes as input the proof data that contain a merkle path to the block where the transaction/receipt originated and a merkle path to the transaction/receipt, also as input the height of the known block to the light client contract needs to be provided, and this block needs to be ahead or on the block of the transaction that we are proving. With all of this the prover can calculate the expected block merkle root and compare it to the one stored in the light client block.

Prerequisite for deploying the prover is that the light_client contract is already deployed
```
cd contracts/prover
./build.sh
./deploy.sh
./deploy_calimero.sh calimero_shard_id
```

# Prerequisites for using the bridge

1. A Calimero shard needs to be running ([contact in case you want to spin up your own Calimero shard](https://www.calimero.network/contact))
2. All contracts need to be deployed on both NEAR and Calimero
3. Relayer from Near to Calimero and a relayer from Calimero to Near need to be running
4. Bridge service that monitors events in realtime on both Near and Calimero need to be running

## Calimero alias

This is a helper alias for [near-cli](https://docs.near.org/tools/near-cli) when interacting with a Calimero shard:
```
alias calimero='function x() { near ${@:2} --nodeUrl https://api.development.calimero.network/api/v1/shards/$1/neard-rpc --networkId $1;} ; x'
```

# Connector contracts

The prover enables us to build a set of contracts for transferring assets from one chain to another. Calimero supports transfering Fungible tokens as well as Non Fungible tokens from one chain to another. Also, via the Calimero bridge cross shard calls can be executed.

## FT connector

With the fungible token connector ft's can be bridged from NEAR to Calimero and back. In order to bridge some ft from NEAR testnet to Calimero, a single transaction needs to be called. Just lock the wanted amount of tokens to the ft connector contract. Once they are transferred, the bridge service and the relayer will be notified about it and try to prove on the Calimero shard that the locking of tokens happened on NEAR. If proved, wrapped tokens are minted on Calimero shard.

Example of locking fungible tokens on NEAR testnet with [transaction on NEAR testnet](https://explorer.testnet.near.org/transactions/FAGjdTYkHJ2bdWP9CRznzxj7kB3KXRfNDw3hsdUGdFZX):
```
near call usdn.testnet ft_transfer_call --args ‘{“receiver_id”:“ft_source_connector.cali99.apptest-development.testnet”,“amount”:“12345",“msg”:“”}’ --accountId igi.testnet --depositYocto 1 --gas 3000000000000
```

If the users want to get the tokens back on the original chain (in this case NEAR testnet), they simply call withdraw on the bridged token on the other chain (in this case Calimero shard) which will essentially burn tokens. The bridge service will be notified via emitted event that a burn happened and try to prove on the ft connector contract on NEAR that a burn happaned on Calimero shard. If proved, tokens are unlocked.

Example of withdrawing/burning tokens on Calimero with [transaction on NEAR testnet](https://explorer.testnet.near.org/transactions/5hnG8P52BrGVeuPR37nHvHz2SKtQabDKyC2yuQmfe34C):
```
calimero cali99-calimero-testnet call usdn.ft_dest_connector.cali99.calimero.testnet withdraw --args '{"amount":"345"}' --accountId igi.testnet --depositYocto 1 --gas 300000000000000
```

Prerequisite for deploying the connectors is that the prover contract on each chain is already deployed
```
cd contracts/ft_bridge_token
./build.sh
./deploy.sh
cd ../bridge_token_deployer
./build_ft.sh
cd ../ft_connector
./build.sh
```

Deploy on both NEAR testnet and Calimero:
```
./deploy.sh
./deploy_calimero.sh calimero_shard_id
```

## NFT connector

With the non fungible token connector nft's can be bridged from NEAR to Calimero and back. In order to bridge some nft from NEAR testnet to Calimero, a single transaction needs to be called. Just lock the wanted token to the nft connector contract. Once the token is transferred, the bridge service will be notified about it and try to prove on the Calimero shard that the locking of tokens happened on NEAR. If proved, wrapped token is minted on Calimero shard.

Example of locking non fungible token on NEAR testnet with [transaction on NEAR testnet](https://explorer.testnet.near.org/transactions/BNSeCbCqUN1WAh5mr7pAnaXsfDWMff4keYzQXZkR4YuP):
```
near call nft-test.igi.testnet nft_transfer_call --args '{"receiver_id":"nft_source_connector.cali99.apptest-development.testnet", "token_id":"0", "msg":""}' --accountId igi.testnet --depositYocto 1 --gas 300000000000000
```

If the users want to get the token back on the original chain (in this case NEAR testnet), they simply call withdraw on the bridged token on the other chain (in this case Calimero shard) which will essentially burn the token. The bridge service will be notified via emitted event that a burn happened and try to prove on the nft connector contract on NEAR that a burn happaned on Calimero shard. If proved, tokens are unlocked.

Example of withdrawing/burning the nft token on Calimero with [transaction on NEAR testnet](https://explorer.testnet.near.org/transactions/87kTM6E4rtbDZtm91hhScPrjbu2GXWdwoBY4LBkDXLUs):
```
calimero cali99-calimero-testnet call nft-test_igi.nft_dest_connector.cali99.calimero.testnet withdraw --args '{"token_id":"0"}' --accountId igi.testnet --depositYocto 1 --gas 300000000000000
```

Prerequisite for deploying the nft connectors is that the prover contract on each chain is already deployed
```
cd contracts/nft_bridge_token
./build.sh
./deploy.sh
cd ../bridge_token_deployer
./build_nft.sh
cd ../nft_connector
./build.sh
```

Deploy on both NEAR testnet and Calimero:
```
./deploy.sh
./deploy_calimero.sh calimero_shard_id
```

## Cross shard call connector

Via the Calimero bridge, cross shard calls can be executed with a callback. Meaning that a contract on Calimero can call into contracts on NEAR and get a callback. Also, contracts on NEAR can call into Calimero contracts and get a callback.

An example DAPP that is deployed on NEAR testnet and makes cross shard calls to Calimero and back is the [tic-tac-toe game whose contracts can be found here](https://github.com/calimero-is-near/calimero-examples/tree/master/tic-tac-toe/contracts)

Once both players register for playing on the contract on NEAR testnet, the game gets started on Calimero, a cross shard call is executed denoting that two players from NEAR testnet want to start a game on Calimero:

```
near call tictactoe.igi.testnet register_player --accountId igi.testnet
near call tictactoe.igi.testnet register_player --accountId mikimaus.testnet
```
Players registered for a game: [playerA](https://explorer.testnet.near.org/transactions/7TscAyfni781qz2vgpeKXTR2Bc8JRXaeftjK1SHJHMNc) and [playerB](https://explorer.testnet.near.org/transactions/8Ex1pojKw8fp5Y8X85gy5fTX9VyWrC4VUHCTvNR1kArT)

Once the second player registered for a game, a ```CALIMERO_EVENT_CROSS_CALL``` event is emitted via which the arguments to call on Calimero shard are transitted. The bridge service gets this events and tries to prove on the cross shard connector on Calimero that a game of tic tac toe needs to start. If proved, ```start_game``` method is called on tic tac toe contract on Calimero. Immediatelly after that a ```CALIMERO_EVENT_CROSS_RESPONSE``` event is emitted from the cross shard connector on Calimero. Once proved on NEAR, the callback method ```game_started``` is called. Here is the transaction showing the [executed callback](https://explorer.testnet.near.org/transactions/DWyCptftairNMtryiikSkWRVadeqSRN8CEgC1eEAZahL).

Similarly, you can see how a ```game_ended``` was called from Calimero to NEAR testnet [here](https://explorer.testnet.near.org/transactions/FkE4dEHzbJ5tZKdrYC3QYbpWf5dGVgsqbqVwb12wUp1z)

And now the final result of the game player on Calimero can be viewed on NEAR testnet:
```
near view tictactoe.igi.testnet get_finished_game --args '{"game_id":0}'
View call: tictactoe.igi.testnet.get_finished_game({"game_id":0})
{
  board: [ [ 'O', 'X', 'X' ], [ 'O', 'U', 'U' ], [ 'O', 'U', 'U' ] ],
  player_a: 'igi.testnet',
  player_b: 'igi.testnet',
  status: 'PlayerAWon',
  player_a_turn: false
}
```

Prerequisite for deploying the cross shard connectors is that the prover contract on each chain is already deployed
```
cd ../xsc_connector
./build.sh
```

Deploy on both NEAR testnet and Calimero:
```
./deploy.sh
./deploy_calimero.sh calimero_shard_id
```

# Permissions contract

It is possible to allow/deny certain accounts from using the bridge. For (n)ft's specific accounts can be denied. For cross shard calls, each account id can be denied per contract as well.

```
cd ../connector_permissions
./build.sh
```

Deploy to both NEAR and Calimero.

To use the permission contract, just provide the contract_id to the init function of each connector.

# WASM Conf Demo

This is a simple smart contract game built on Efinity that demonstrates the following things:
- Simple turn-based combat
- Randomly generated values for turn order and attacks
- Minting, burning, and transferring tokens
- Using NFTs as equipment and freezing the tokens while they're in use
- Storing a strength value as metadata on an NFT
- Encoding additional data into a `TokenId` (see `WrappedTokenId`)
- A fungible token used as a currency for buying items in-game
- A game config that can be modified during the game
- Events for game actions
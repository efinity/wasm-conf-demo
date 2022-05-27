#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
mod types;

use efinity_contracts::{prelude::*, Freeze, MintRecipient, ReserveIdentifier, TransferRecipient};
use ink::codegen::Env;
use ink_lang as ink;
use ink_prelude::vec::Vec;
use ink_storage::traits::SpreadAllocate;
use ink_storage::Mapping;
use types::*;

const EQUIPMENT_ATTRIBUTE_KEY: &[u8; 9] = b"equipment";
// static EQUIPMENT_ATTRIBUTE_KEY: AttributeKey = b"equipment".to_vec();

/// Multi-Tokens example smart contract
#[ink::contract(env = EfinityEnvironment)]
mod game {
    use super::*;
    use scale::Decode;
    use scale::Encode;

    /// Error types for the game
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// The caller does not have permission for this operation
        NoPermission,
    }

    /// Result type for the game
    pub type Result<T> = core::result::Result<T, Error>;

    /// The storage for this contract
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Game {
        config: Config,
        owner: AccountId,
        /// The id of the collection used for all tokens
        next_token_id: TokenId,
        heroes: Mapping<AccountId, Hero>,
        battles: Mapping<BattleId, Battle>,
    }

    impl Game {
        /// Create a new game instance
        #[ink(constructor)]
        pub fn new(config: Config) -> Self {
            ink::utils::initialize_contract(|contract: &mut Self| {
                contract.owner = Self::env().caller();
                contract.config = config;
            })
        }

        /// Modify the configuration of the game
        #[ink(message)]
        pub fn mutate_config(&mut self, mutation: ConfigMutation) -> Result<()> {
            // make sure the owner is the caller
            if self.env().caller() != self.owner {
                return Err(Error::NoPermission);
            }

            // mutate the fields that have values
            if let Some(collection_id) = mutation.collection_id {
                self.config.collection_id = collection_id;
            }
            if let Some(currency_token_id) = mutation.gold_token_id {
                self.config.gold_token_id = currency_token_id;
            }
            if let Some(initial_token_id) = mutation.initial_token_id {
                self.config.initial_token_id = initial_token_id;
            }
            if let Some(initial_hero_health) = mutation.initial_hero_health {
                self.config.initial_hero_health = initial_hero_health;
            }
            Ok(())
        }

        /// Create a hero for the caller
        #[ink(message)]
        pub fn create_hero(&mut self) -> Hero {
            let caller = self.env().caller();

            // mint the equipment tokens
            let token_ids = self.mint_nfts(2);

            // set the metadata of the equipment
            let weapon_token_id = token_ids[0];
            let armor_token_id = token_ids[1];
            // let bytes = [1];
            // let (value, _) = self.env().random(&bytes);
            // let value = value[0];
            // let random_number_in_range = value[0] % 100;
            // get more bytes if I want a bigger range
            // hash is a [u8; 32]
            let metadata = TokenMetadata {
                token_type: TokenType::Weapon,
                value: 0,
            };
            self.env().extension().set_attribute(
                self.config.collection_id,
                Some(weapon_token_id),
                EQUIPMENT_ATTRIBUTE_KEY.to_vec(),
                Some(metadata.encode()),
                // metadata.encode(),
            );

            // create hero with the tokens we just minted
            let hero = Hero::new(
                self.config.initial_hero_health,
                Some(weapon_token_id),
                Some(armor_token_id),
            );
            self.heroes.insert(caller, &hero);
            hero
        }

        /// Start a battle with a randomly generated enemy
        #[ink(message)]
        pub fn start_battle(&mut self) -> BattleId {
            todo!()
        }

        /// Advance the battle to the next turn
        #[ink(message)]
        pub fn advance_battle(&mut self, battle_id: BattleId, command: Command) {
            todo!()
        }

        /// Change the equipment of the caller's hero
        #[ink(message)]
        pub fn change_equipment(&mut self, weapon: Option<TokenId>, armor: Option<TokenId>) {
            todo!()
        }

        /// Purchase a healing potion
        #[ink(message)]
        pub fn buy_potion(&mut self, quantity: u8) {
            // can only buy if you're not in any battles
            todo!()
        }

        /// Purchase an equipment upgrade for `token_id`. It must be owned by the
        /// caller.
        #[ink(message)]
        pub fn upgrade_equipment(&mut self, token_id: TokenId) {}
    }

    // helper functions
    impl Game {
        fn increment_next_token_id(&mut self) -> TokenId {
            let token_id = self.next_token_id;
            self.next_token_id += 1;
            token_id
        }

        fn mint_nfts(&mut self, count: usize) -> Vec<TokenId> {
            let caller = self.env().caller();
            let token_ids: Vec<_> = (0..count).map(|_| self.increment_next_token_id()).collect();
            for token_id in &token_ids {
                let params = MintParams::CreateToken {
                    token_id: *token_id,
                    initial_supply: 1,
                    unit_price: 100_000_000_000_000_000,
                    // unit_price: self.env().extension().get_token_account_deposit(),
                    cap: None,
                    // cap: Some(TokenCap::SingleMint),
                };
                // let encoded = (caller, self.collection_id, params.clone()).encode();
                // println!(
                //     "encoded len: {}, first: {}, last: {}",
                //     encoded.len(),
                //     encoded[0],
                //     encoded[encoded.len() - 1]
                // );
                self.env()
                    .extension()
                    .mint(caller, self.config.collection_id, params.clone());
            }
            token_ids
        }

        fn generate_random_number(&self, end: u8) -> u8 {
            let bytes = [1];
            // let (hash, _) = self.env().random(&bytes);
            todo!()
            // self.hash_to_ranged_number(hash, end)
        }

        fn hash_to_ranged_number(&self, hash: [u8;32], end: u8) -> u8 {
            let value = hash[0];
            value % end
        }
    }

    #[cfg(test)]
    pub mod tests {
        use super::*;
        use crate::mock::Token;
        use ink_env::Error::Decode;
        use ink_env::{caller, test};
        use mock::MockChainExtension;
        use scale::Encode;
        use std::cell::RefCell;
        use std::collections::HashMap;

        thread_local! {
            pub static MOCK_EFINITY: RefCell<MockChainExtension> = RefCell::new(Default::default());
        }

        /// Test that the `create_hero` function is working
        #[ink::test]
        fn test_create_hero() {
            // init game
            let collection_id = 1;
            let config = Config {
                collection_id,
                gold_token_id: 0,
                initial_token_id: 1,
                initial_hero_health: 100,
            };
            let mut game = Game::new(config);
            mock::register_chain_extension();

            // create a hero for bob
            let accounts = test::default_accounts::<EfinityEnvironment>();
            test::set_caller::<EfinityEnvironment>(accounts.bob);
            let hero = game.create_hero();

            // verify the hero's tokens for weapon and armor were minted
            assert_eq!(hero, game.heroes.get(accounts.bob).unwrap());
            MOCK_EFINITY.with(|efinity| {
                let efinity = efinity.borrow();
                let weapon_id = hero.weapon_token_id.unwrap();
                let armor_id = hero.armor_token_id.unwrap();

                // assert tokens exist
                assert!(efinity.token_of(collection_id, weapon_id).is_some());
                assert!(efinity.token_of(collection_id, armor_id).is_some());

                // assert bob has the tokens
                println!(
                    "token account: {:?}",
                    efinity.token_account_of(collection_id, weapon_id, accounts.alice)
                );
                assert_eq!(
                    efinity.balance_of(collection_id, weapon_id, accounts.bob),
                    1
                );
                assert_eq!(efinity.balance_of(collection_id, armor_id, accounts.bob), 1);
            })
        }

        #[ink::test]
        fn test_generate_random_number() {
            let game = Game::new(Default::default());
            let mut hash = [0_u8;32];
            let random_number = game.hash_to_ranged_number(hash, 100);
            assert_eq!(random_number, 0);
            hash[0] = 126;
            assert_eq!(game.hash_to_ranged_number(hash, 100), 50);
        }
    }
}

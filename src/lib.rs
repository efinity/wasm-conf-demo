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

fn attribute_key() -> AttributeKey {
    EQUIPMENT_ATTRIBUTE_KEY.to_vec()
}

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
        /// The equipment being equipped is invalid
        InvalidEquipment,
        AttributeDecodeFailed,
        HeroNotFound,
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
        random_nonce: u32,
        random_seed: u32,
        heroes: Mapping<AccountId, Hero>,
    }

    impl Game {
        /// Create a new game instance
        #[ink(constructor)]
        pub fn new(config: Config, random_seed: u32) -> Self {
            ink::utils::initialize_contract(|contract: &mut Self| {
                contract.owner = Self::env().caller();
                contract.config = config;
                contract.random_seed = random_seed;
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

            // add attribute to equipment tokens
            let weapon_token_id = token_ids[0];
            let armor_token_id = token_ids[1];
            self.add_equipment_attribute(
                weapon_token_id,
                TokenType::Weapon,
                self.config.initial_hero_stats_range,
            );
            self.add_equipment_attribute(
                armor_token_id,
                TokenType::Hat,
                self.config.initial_hero_stats_range,
            );

            // create hero with the tokens we just minted
            let hero = Hero::new(
                self.config.initial_hero_health,
                weapon_token_id,
                Some(armor_token_id),
            );
            self.heroes.insert(caller, &hero);
            hero
        }

        /// Start a battle with a randomly generated enemy
        #[ink(message)]
        pub fn start_battle(&mut self) -> Result<()> {
            let hero = self
                .heroes
                .get(self.env().caller())
                .ok_or(Error::HeroNotFound)?;
            Ok(())
        }

        /// Advance the battle to the next turn
        #[ink(message)]
        pub fn advance_battle(&mut self, command: Command) {
            todo!()
        }

        /// Change the equipment of the caller's hero
        #[ink(message)]
        pub fn equip(&mut self, token_id: TokenId) -> Result<()> {
            let caller = self.env().caller();
            let mut hero = self.heroes.get(caller).ok_or(Error::HeroNotFound)?;

            // get the metadata
            let attribute = self
                .env()
                .extension()
                .attribute_of(self.config.collection_id, Some(token_id), attribute_key())
                .ok_or(Error::InvalidEquipment)?;
            let metadata: TokenMetadata = Decode::decode(&mut &attribute.value[..])
                .map_err(|_| Error::AttributeDecodeFailed)?;

            match metadata.token_type {
                TokenType::Weapon => hero.weapon_id = token_id,
                TokenType::Hat => hero.hat_id = Some(token_id),
            }
            self.heroes.insert(caller, &hero);
            Ok(())
        }

        /// Remove the caller's hat
        #[ink(message)]
        pub fn unequip_hat(&mut self) {}

        /// Purchase a healing potion
        #[ink(message)]
        pub fn buy_potion(&mut self, quantity: u8) {
            // can only buy if you're not in a battle
            todo!()
        }

        /// Buy a new weapon. It will be equipped if `equip` is true.
        #[ink(message)]
        pub fn buy_weapon(&mut self, equip: bool) {
            // can only buy if you're not in a battle
            todo!()
        }
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
                self.env()
                    .extension()
                    .mint(caller, self.config.collection_id, params.clone());
            }
            token_ids
        }

        fn add_equipment_attribute(
            &mut self,
            token_id: TokenId,
            token_type: TokenType,
            value_range: Range<u32>,
        ) {
            let metadata = TokenMetadata {
                token_type,
                value: self.random_in_range(value_range),
            };
            self.env().extension().set_attribute(
                self.config.collection_id,
                Some(token_id),
                attribute_key(),
                metadata.encode(),
            );
        }

        fn random_in_range(&mut self, range: Range<u32>) -> u32 {
            let mut subject = [0_u8; 12];
            subject[0..4].copy_from_slice(&self.random_seed.to_le_bytes());
            subject[4..8].copy_from_slice(&self.random_nonce.to_le_bytes());
            subject[8..12].copy_from_slice(&self.env().block_number().to_le_bytes());
            self.random_nonce += 1;
            let (hash, _) = self.env().random(&subject);
            let mut bytes = [0_u8; 4];
            bytes.copy_from_slice(&hash.as_ref()[0..4]);
            let random_number = u32::from_le_bytes(bytes);
            lerp(range.start, range.end, random_number)
        }
    }

    /// Linearly interpolates between `a` and `b` by `t`, where `t` is considered
    /// a fraction of its max value
    fn lerp(a: u32, b: u32, t: u32) -> u32 {
        let input = (t as u64) * 1000;
        let fraction = input / u32::MAX as u64;
        let output = ((fraction * b as u64) / 1000) + a as u64;
        output as u32
    }

    #[cfg(test)]
    pub mod tests {
        use super::*;
        use crate::mock::Token;
        use efinity_contracts::AccountId;
        use ink_env::Error::Decode;
        use ink_env::{caller, test};
        use mock::MockChainExtension;
        use scale::Encode;
        use std::cell::RefCell;
        use std::collections::HashMap;

        thread_local! {
            pub static MOCK_EFINITY: RefCell<MockChainExtension> = RefCell::new(Default::default());
        }

        fn accounts() -> test::DefaultAccounts<EfinityEnvironment> {
            test::default_accounts()
        }

        fn alice() -> AccountId {
            accounts().alice
        }

        /// Test that the `create_hero` function is working
        #[ink::test]
        fn test_create_hero() {
            // init game
            let config = Config::default();
            let collection_id = config.collection_id;
            let mut game = Game::new(config, 0);
            mock::register_chain_extension();

            // create a hero for bob
            let accounts = test::default_accounts::<EfinityEnvironment>();
            test::set_caller::<EfinityEnvironment>(accounts.bob);
            let hero = game.create_hero();

            // verify the hero's tokens for weapon and armor were minted
            assert_eq!(hero, game.heroes.get(accounts.bob).unwrap());
            MOCK_EFINITY.with(|efinity| {
                let efinity = efinity.borrow();
                let weapon_id = hero.weapon_id;
                let armor_id = hero.hat_id.unwrap();

                // assert tokens exist
                assert!(efinity.token_of(collection_id, weapon_id).is_some());
                assert!(efinity.token_of(collection_id, armor_id).is_some());

                // assert bob has the tokens
                assert_eq!(
                    efinity.balance_of(collection_id, weapon_id, accounts.bob),
                    1
                );
                assert_eq!(efinity.balance_of(collection_id, armor_id, accounts.bob), 1);

                // assert attributes exist
                assert!(efinity
                    .attribute_of(
                        collection_id,
                        Some(weapon_id),
                        EQUIPMENT_ATTRIBUTE_KEY.to_vec()
                    )
                    .is_some());
                assert!(efinity
                    .attribute_of(
                        collection_id,
                        Some(armor_id),
                        EQUIPMENT_ATTRIBUTE_KEY.to_vec()
                    )
                    .is_some());
            })
        }

        #[ink::test]
        fn test_set_hero_equipment() {
            mock::register_chain_extension();
            let mut game = Game::new(Default::default(), 0);
            game.create_hero();

            // cannot set to token that does not exist
            assert_eq!(game.equip(10).unwrap_err(), Error::InvalidEquipment);

            // Mint some NFTs. It will still fail because there is no weapon attribute
            let token_ids = game.mint_nfts(2);
            let weapon_id = token_ids[0];
            assert_eq!(game.equip(weapon_id).unwrap_err(), Error::InvalidEquipment);

            // add equipment attribute and now it works
            game.add_equipment_attribute(weapon_id, TokenType::Weapon, (1, 1).into());
            game.equip(weapon_id).unwrap();
            let hero = game.heroes.get(alice()).unwrap();
            assert_eq!(hero.weapon_id, weapon_id);
        }

        #[test]
        fn test_lerp() {
            assert_eq!(lerp(0, 100, u32::MAX), 100);
            assert_eq!(lerp(0, 100, (u32::MAX / 2) + 1), 50);
            assert_eq!(lerp(0, 100, (u32::MAX / 10) + 1), 10);
            assert_eq!(lerp(5, 100, 0), 5);
        }

        #[ink::test]
        fn test_generate_random_number() {
            // let game = Game::new(Default::default());
            // let mut hash = [0_u8; 32];
            // let random_number = game.hash_to_ranged_number(hash, 100);
            // assert_eq!(random_number, 0);
            // hash[0] = 126;
            // assert_eq!(game.hash_to_ranged_number(hash, 100), 50);
        }
    }
}

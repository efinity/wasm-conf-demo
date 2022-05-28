#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
mod types;

use efinity_contracts::{prelude::*, Freeze, MintRecipient, ReserveIdentifier, TransferRecipient};
use ink::codegen::Env;
use ink_lang as ink;
use ink_prelude::vec::Vec;
use ink_storage::{traits::SpreadAllocate, Mapping};
use types::*;

/// The attribute key used for equipment
fn attribute_key() -> AttributeKey {
    b"equipment".to_vec()
}

/// Multi-Tokens example smart contract
#[ink::contract(env = EfinityEnvironment)]
mod game {
    use super::*;
    use scale::{Decode, Encode};

    /// Error types for the game
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// The caller is not allowed to call this function
        NotAllowed,
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

        /// Returns the game's config
        #[ink(message)]
        pub fn get_config(&self) -> Config {
            self.config.clone()
        }

        /// Modify the configuration of the game. Only callable by the owner.
        #[ink(message)]
        pub fn mutate_config(&mut self, mutation: ConfigMutation) -> Result<()> {
            // make sure the owner is the caller
            if self.env().caller() != self.owner {
                return Err(Error::NotAllowed);
            }
            mutation.apply_to(&mut self.config);
            Ok(())
        }

        /// Create a hero for the caller
        #[ink(message)]
        pub fn create_hero(&mut self) -> Hero {
            let caller = self.env().caller();

            // mint the weapon token
            let weapon_id = self.mint_nft();

            // add attribute to equipment tokens
            self.add_equipment_attribute(
                weapon_id,
                TokenType::Weapon,
                Some(self.config.initial_hero_stats_range),
            );

            // create hero with the token we just minted
            let hero = Hero::new(self.config.initial_hero_health, weapon_id);
            self.heroes.insert(caller, &hero);
            hero
        }

        /// Start a battle with a randomly generated enemy
        #[ink(message)]
        pub fn start_battle(&mut self) -> Result<()> {
            let mut hero = self
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

        fn mint_nft(&mut self) -> TokenId {
            let caller = self.env().caller();
            let token_id = self.increment_next_token_id();
            let params = MintParams::CreateToken {
                token_id,
                initial_supply: 1,
                // unit_price: 100_000_000_000_000_000,
                unit_price: self.env().extension().get_token_account_deposit(),
                // cap: None,
                cap: Some(TokenCap::SingleMint),
            };
            self.env()
                .extension()
                .mint(caller, self.config.collection_id, params.clone());
            token_id
        }

        fn add_equipment_attribute(
            &mut self,
            token_id: TokenId,
            token_type: TokenType,
            value_range: Option<Range<u32>>,
        ) {
            let metadata = TokenMetadata {
                token_type,
                value: value_range
                    .map(|x| self.random_in_range(x))
                    .unwrap_or_default(),
            };
            self.env().extension().set_attribute(
                self.config.collection_id,
                Some(token_id),
                attribute_key(),
                metadata.encode(),
            );
        }

        fn generate_enemy(&mut self) -> Enemy {
            let hat_id = {
                if self.random_in_range((0, 100).into()) >= self.config.enemy_wearing_hat_chance {
                    let hat_id = self.mint_nft();
                    self.add_equipment_attribute(hat_id, TokenType::Hat, None);
                    Some(hat_id)
                } else {
                    None
                }
            };
            Enemy {
                hat_id,
                health: self.random_in_range(self.config.enemy_health_range),
                strength: self.random_in_range(self.config.enemy_strength_range),
            }
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
        const PRECISION: u64 = 1000;
        let input = (t as u64) * PRECISION;
        let fraction = input / u32::MAX as u64;
        let output = ((fraction * b as u64) / PRECISION) + a as u64;
        output as u32
    }

    #[cfg(test)]
    pub mod tests {
        use super::*;
        use crate::mock::Token;
        use efinity_contracts::AccountId;
        use ink_env::{caller, test, Error::Decode};
        use mock::MockChainExtension;
        use scale::Encode;
        use std::{cell::RefCell, collections::HashMap};

        thread_local! {
            pub static MOCK_EFINITY: RefCell<MockChainExtension> = RefCell::new(Default::default());
        }

        fn accounts() -> test::DefaultAccounts<EfinityEnvironment> {
            test::default_accounts()
        }
        fn alice() -> AccountId {
            accounts().alice
        }
        fn bob() -> AccountId {
            accounts().bob
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
            test::set_caller::<EfinityEnvironment>(bob());
            let hero = game.create_hero();

            // verify the hero's tokens for weapon and armor were minted
            assert_eq!(hero, game.heroes.get(bob()).unwrap());
            MOCK_EFINITY.with(|efinity| {
                let efinity = efinity.borrow();

                // assert weapon token exists
                assert!(efinity.token_of(collection_id, hero.weapon_id).is_some());

                // assert bob has the weapon token
                assert_eq!(efinity.balance_of(collection_id, hero.weapon_id, bob()), 1);

                // assert the weapon token attribute exists
                assert!(efinity
                    .attribute_of(collection_id, Some(hero.weapon_id), attribute_key())
                    .is_some());
            })
        }

        #[ink::test]
        fn test_mutate_config() {
            let initial_config = Config::default();
            let mut game = Game::new(initial_config.clone(), 0);
            let mutation = ConfigMutation {
                potion_cost: Some(1000),
                ..Default::default()
            };
            game.mutate_config(mutation).unwrap();
            let mut config = game.get_config();

            // make sure the potion cost changed
            assert_eq!(config.potion_cost, 1000);

            // if we change the potion cost back, everything else should be the same
            config.potion_cost = initial_config.potion_cost;
            assert_eq!(initial_config, config);
        }

        #[ink::test]
        fn test_set_hero_equipment() {
            mock::register_chain_extension();
            let mut game = Game::new(Default::default(), 0);
            game.create_hero();

            // cannot set to token that does not exist
            assert_eq!(game.equip(10).unwrap_err(), Error::InvalidEquipment);

            // Mint some NFTs. It will still fail because there is no weapon attribute
            let weapon_id = game.mint_nft();
            assert_eq!(game.equip(weapon_id).unwrap_err(), Error::InvalidEquipment);

            // add equipment attribute and now it works
            game.add_equipment_attribute(weapon_id, TokenType::Weapon, Some((1, 1).into()));
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
    }
}

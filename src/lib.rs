#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
mod types;

use efinity_contracts::{
    prelude::*, AccountId, Freeze, MintRecipient, ReserveIdentifier, TransferRecipient,
};
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
    use efinity_contracts::FreezeType;
    use ink_env::test;
    use scale::{Decode, Encode};

    #[ink(event)]
    pub struct HeroCreated {
        pub account_id: AccountId,
        pub weapon_id: TokenId,
        pub weapon_strength: u32,
    }

    #[ink(event)]
    pub struct WeaponPurchased {
        pub token_id: TokenId,
        pub strength: u32,
    }

    #[ink(event)]
    pub struct BattleAdvanced {
        pub hero_id: AccountId,
        pub round_number: u32,
        pub hero_damage_received: u32,
        pub enemy_damage_received: u32,
    }

    #[ink(event)]
    pub struct BattleEnded {
        /// The `AccountId` of the hero
        pub hero_id: AccountId,
        /// True if the hero won the battle
        pub hero_wins: bool,
    }

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
        /// This operation is not allowed while in battle
        HeroIsInBattle,
        /// This operation is only allowed while in battle
        HeroNotInBattle,
        /// The hero does not have any potions
        HeroHasNoPotions,
        /// Not enough gold
        NotEnoughGold,
    }

    /// Result type for the game
    pub type Result<T> = core::result::Result<T, Error>;

    /// The storage for this contract
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Game {
        config: Config,
        owner: AccountId,
        collection_id: CollectionId,
        gold_token_id: TokenId,
        /// The id of the collection used for all tokens
        next_token_id: TokenId,
        random_nonce: u32,
        random_seed: u32,
        heroes: Mapping<AccountId, Hero>,
    }

    impl Game {
        /// Create a new game instance
        #[ink(constructor)]
        pub fn new(
            collection_id: CollectionId,
            gold_token_id: TokenId,
            initial_token_id: TokenId,
            random_seed: u32,
            config: Option<Config>,
        ) -> Self {
            assert_ne!(
                gold_token_id, initial_token_id,
                "gold_token_id and initial_token_id must be different"
            );

            ink::utils::initialize_contract(|contract: &mut Self| {
                contract.owner = Self::env().caller();
                contract.collection_id = collection_id;
                contract.gold_token_id = gold_token_id;
                contract.next_token_id = initial_token_id;
                contract.config = config.unwrap_or_default();
                contract.random_seed = random_seed;
            })
        }

        /// Returns the game's config
        #[ink(message)]
        pub fn get_config(&self) -> Config {
            self.config.clone()
        }

        /// Returns the `Hero` for `account_id`
        #[ink(message)]
        pub fn get_hero(&self, account_id: AccountId) -> Option<Hero> {
            self.heroes.get(account_id)
        }

        #[ink(message)]
        pub fn get_metadata(&self, token_id: TokenId) -> Result<Option<TokenMetadata>> {
            if let Some(attribute) = self.env().extension().attribute_of(
                self.collection_id,
                Some(token_id),
                attribute_key(),
            ) {
                Ok(Some(
                    Decode::decode(&mut &attribute.value[..])
                        .map_err(|_| Error::AttributeDecodeFailed)?,
                ))
            } else {
                Ok(None)
            }
        }

        #[ink(message)]
        pub fn get_gold_balance(&self, account_id: AccountId) -> TokenBalance {
            self.env()
                .extension()
                .balance_of(self.collection_id, self.gold_token_id, account_id)
        }

        /// Modify the configuration of the game. Only callable by the owner.
        #[ink(message)]
        pub fn mutate_config(&mut self, mutation: ConfigMutation) -> Result<()> {
            // make sure the owner is the caller
            if self.env().caller() != self.owner {
                return Err(Error::NoPermission);
            }
            mutation.apply_to(&mut self.config);
            Ok(())
        }

        /// Create a hero for the caller
        #[ink(message)]
        pub fn create_hero(&mut self) -> Hero {
            let caller = self.env().caller();

            // mint the weapon token
            let weapon_id = self.mint_nft(caller, true);

            // add attribute to equipment tokens
            let weapon_strength = self.add_equipment_attribute(
                weapon_id,
                TokenType::Weapon,
                Some(self.config.starting_weapon_strength_range),
            );

            // create hero with the token we just minted
            let hero = Hero::new(self.config.hero_max_health, weapon_id);
            self.heroes.insert(caller, &hero);

            self.env().emit_event(HeroCreated {
                account_id: caller,
                weapon_id,
                weapon_strength,
            });
            hero
        }

        /// Start a battle with a randomly generated enemy
        #[ink(message)]
        pub fn start_battle(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let mut hero = self.heroes.get(caller).ok_or(Error::HeroNotFound)?;
            let enemy = self.generate_enemy();
            hero.battle = Some(Battle::new(enemy));
            self.heroes.insert(caller, &hero);
            Ok(())
        }

        /// Advance the battle to the next turn
        #[ink(message)]
        pub fn advance_battle(&mut self, command: Command) -> Result<()> {
            /// Returns true if the battle is over
            fn battle_is_over(hero: &Hero, battle: &Battle) -> bool {
                hero.is_dead() || battle.enemy.is_dead()
            }

            let caller = self.env().caller();
            let mut hero = self.heroes.get(caller).ok_or(Error::HeroNotFound)?;
            let mut battle = hero.battle.ok_or(Error::HeroNotInBattle)?;
            let hero_initial_health = hero.health;
            let enemy_initial_health = battle.enemy.health;

            let hero_goes_first = self.random_chance(self.config.hero_goes_first_chance);
            if hero_goes_first {
                self.hero_action(&mut hero, &mut battle, command)?;
                if !battle_is_over(&hero, &battle) {
                    self.enemy_action(&mut hero, &mut battle)?;
                }
            } else {
                self.enemy_action(&mut hero, &mut battle)?;
                if !battle_is_over(&hero, &battle) {
                    self.hero_action(&mut hero, &mut battle, command)?;
                }
            }

            // send the advanced event
            self.env().emit_event(BattleAdvanced {
                hero_id: caller,
                round_number: battle.round_number,
                hero_damage_received: hero_initial_health.saturating_sub(hero.health),
                enemy_damage_received: enemy_initial_health.saturating_sub(battle.enemy.health),
            });
            battle.round_number = battle.round_number.saturating_add(1);

            if battle_is_over(&hero, &battle) {
                hero.battle = None;

                if battle.enemy.is_dead() {
                    let gold_amount = self.random_in_range(self.config.enemy_gold_drop_range);
                    self.mint_gold(gold_amount as TokenBalance);
                    if let Some(hat_id) = battle.enemy.hat_id {
                        self.env().extension().transfer(
                            caller,
                            self.collection_id,
                            TransferParams::Simple {
                                token_id: hat_id,
                                amount: 1,
                                keep_alive: false,
                            },
                        )
                    }
                }
                if hero.is_dead() {
                    hero.health = self.config.hero_max_health;
                    hero.consecutive_victory_count = 0;
                }

                self.env().emit_event(BattleEnded {
                    hero_id: caller,
                    hero_wins: !hero.is_dead(),
                });
            } else {
                hero.battle = Some(battle);
            }
            self.heroes.insert(caller, &hero);
            Ok(())
        }

        #[ink(message)]
        pub fn get_top_heroes(&self) -> Vec<Hero> {
            todo!();
        }

        /// Change the equipment of the caller's hero
        #[ink(message)]
        pub fn equip(&mut self, token_id: TokenId) -> Result<()> {
            let caller = self.env().caller();
            let mut hero = self.heroes.get(caller).ok_or(Error::HeroNotFound)?;

            // get the token metadata
            let metadata = self
                .get_metadata(token_id)?
                .ok_or(Error::InvalidEquipment)?;

            // set eqiupment and prepare thaw
            let mut thaw_token_id: Option<TokenId> = None;
            match metadata.token_type {
                TokenType::Weapon => {
                    thaw_token_id = Some(hero.weapon_id);
                    hero.weapon_id = token_id;
                }
                TokenType::Hat => {
                    thaw_token_id = hero.hat_id;
                    hero.hat_id = Some(token_id)
                }
            }

            // thaw previous token if needed
            if let Some(thaw_token_id) = thaw_token_id {
                self.env().extension().thaw(Freeze {
                    collection_id: self.collection_id,
                    freeze_type: FreezeType::Token(thaw_token_id),
                });
            }

            // freeze new equipped token
            self.env().extension().freeze(Freeze {
                collection_id: self.collection_id,
                freeze_type: FreezeType::Token(token_id),
            });

            // update the hero
            self.heroes.insert(caller, &hero);

            Ok(())
        }

        /// Remove the caller's hat
        #[ink(message)]
        pub fn unequip_hat(&mut self) {}

        #[ink(message)]
        pub fn rest(&mut self) -> Result<()> {
            let mut hero = self.spend_gold(self.config.rest_cost)?;

            // set health to max
            hero.health = self.config.hero_max_health;
            self.heroes.insert(self.env().caller(), &hero);

            Ok(())
        }

        /// Purchase a healing potion
        #[ink(message)]
        pub fn buy_potion(&mut self, quantity: u32) -> Result<()> {
            let mut hero = self.spend_gold(self.config.rest_cost)?;

            // add the potions
            hero.potion_count = hero.potion_count.saturating_add(quantity);
            self.heroes.insert(self.env().caller(), &hero);

            Ok(())
        }

        /// Buy a new weapon.
        /// Returns the `TokenId` of the generated weapon.
        #[ink(message)]
        pub fn buy_weapon(&mut self) -> Result<TokenId> {
            self.spend_gold(self.config.weapon_cost)?;

            // generate the weapon
            let token_id = self.mint_nft(self.env().caller(), false);
            let strength = self.add_equipment_attribute(
                token_id,
                TokenType::Weapon,
                Some(self.config.purchased_weapon_strength_range),
            );
            self.env()
                .emit_event(WeaponPurchased { token_id, strength });

            Ok(token_id)
        }
    }

    // helper functions
    impl Game {
        fn increment_next_token_id(&mut self) -> TokenId {
            let token_id = self.next_token_id;
            self.next_token_id += 1;
            token_id
        }

        fn mint_nft(&mut self, recipient: AccountId, freeze: bool) -> TokenId {
            let token_id = self.increment_next_token_id();
            let params = MintParams::CreateToken {
                token_id,
                initial_supply: 1,
                unit_price: self.env().extension().get_token_account_deposit(),
                cap: Some(TokenCap::SingleMint),
            };
            self.env()
                .extension()
                .mint(recipient, self.collection_id, params.clone());
            if freeze {
                self.env().extension().freeze(Freeze {
                    collection_id: self.collection_id,
                    freeze_type: FreezeType::Token(token_id),
                })
            }
            token_id
        }

        fn mint_gold(&mut self, amount: TokenBalance) {
            let params = MintParams::Mint {
                token_id: self.gold_token_id,
                amount,
                unit_price: None,
            };
            self.env()
                .extension()
                .mint(self.env().caller(), self.collection_id, params);
        }

        fn burn_gold(&mut self, amount: TokenBalance) {
            let params = BurnParams {
                token_id: self.gold_token_id,
                amount,
                keep_alive: false,
                remove_token_storage: false,
            };
            self.env().extension().burn(self.collection_id, params);
        }

        fn add_equipment_attribute(
            &mut self,
            token_id: TokenId,
            token_type: TokenType,
            value_range: Option<Range>,
        ) -> u32 {
            let strength = value_range
                .map(|x| self.random_in_range(x))
                .unwrap_or_default();
            let metadata = TokenMetadata {
                token_type,
                strength,
            };
            self.env().extension().set_attribute(
                self.collection_id,
                Some(token_id),
                attribute_key(),
                metadata.encode(),
            );
            strength
        }

        fn spend_gold(&mut self, cost: TokenBalance) -> Result<Hero> {
            let caller = self.env().caller();

            // make sure hero is not in a battle
            let hero = self.get_hero(caller).ok_or(Error::HeroNotFound)?;
            if hero.battle.is_some() {
                return Err(Error::HeroIsInBattle);
            }

            // check the balance
            let gold_balance =
                self.env()
                    .extension()
                    .balance_of(self.collection_id, self.gold_token_id, caller);
            if gold_balance < cost {
                return Err(Error::NotEnoughGold);
            }

            // burn the gold being spent
            self.burn_gold(self.config.rest_cost);

            Ok(hero)
        }

        fn generate_enemy(&mut self) -> Enemy {
            let hat_id = {
                if self.random_chance(self.config.enemy_wearing_hat_chance) {
                    // the hat is owned by the contract
                    let hat_id = self.mint_nft(self.env().account_id(), false);
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

        fn hero_action(
            &mut self,
            hero: &mut Hero,
            battle: &mut Battle,
            command: Command,
        ) -> Result<()> {
            match command {
                Command::Attack => {
                    let metadata = self
                        .get_metadata(hero.weapon_id)?
                        .ok_or(Error::InvalidEquipment)?;
                    let strength = metadata.strength;
                    battle.enemy.health = battle.enemy.health.saturating_sub(strength);
                }
                Command::Heal => {
                    if hero.potion_count == 0 {
                        return Err(Error::HeroHasNoPotions);
                    }
                    hero.health = self.config.hero_max_health;
                    hero.potion_count = hero.potion_count.saturating_sub(1);
                }
            }
            Ok(())
        }

        fn enemy_action(&mut self, hero: &mut Hero, battle: &mut Battle) -> Result<()> {
            let enemy = &mut battle.enemy;
            hero.health = hero.health.saturating_sub(enemy.strength);
            Ok(())
        }

        fn random_in_range(&mut self, range: Range) -> u32 {
            // create the subject
            let mut subject = [0_u8; 12];
            subject[0..4].copy_from_slice(&self.random_seed.to_le_bytes());
            subject[4..8].copy_from_slice(&self.random_nonce.to_le_bytes());
            subject[8..12].copy_from_slice(&self.env().block_number().to_le_bytes());

            // add to the nonce because we used it
            self.random_nonce += 1;

            // get random hash
            let (hash, _) = self.env().random(&subject);

            // create a number from the hash
            let mut bytes = [0_u8; 4];
            bytes.copy_from_slice(&hash.as_ref()[0..4]);
            let random_number = u32::from_le_bytes(bytes);

            // linearly interpolate the number to the range
            lerp(range.start, range.end, random_number)
        }

        fn random_chance(&mut self, chance: u32) -> bool {
            self.random_in_range((0, 100).into()) <= chance
        }
    }

    /// Linearly interpolates between `a` and `b` by `t`, where `t` is considered
    /// a fraction of its max value
    fn lerp(a: u32, b: u32, t: u32) -> u32 {
        const PRECISION: u64 = 100;
        let input = (t as u64) * PRECISION;
        let fraction = input / u32::MAX as u64;
        let length: u64 = b as u64 - a as u64;
        let output = ((fraction * length) / PRECISION) + a as u64;
        // println!("a: {}, b: {}, t: {}, output: {}", a, b, t, output);
        output as u32
    }

    #[cfg(test)]
    pub mod tests {
        use super::*;
        use crate::mock::Token;
        use efinity_contracts::AccountId;
        use ink_env::{caller, test};
        use mock::MockChainExtension;
        use scale::Encode;
        use std::{cell::RefCell, collections::HashMap};

        thread_local! {
            pub static MOCK_EFINITY: RefCell<MockChainExtension> = RefCell::new(Default::default());
        }

        fn init_game(config: Config) -> Game {
            mock::register_chain_extension();
            let game = Game::new(1000, 0, 1, 0, Some(config));
            MOCK_EFINITY.with(|efinity| {
                efinity.borrow_mut().contract_address = game.env().account_id();
            });
            game
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
            let mut game = init_game(config);
            let collection_id = game.collection_id;

            // create a hero for bob
            test::set_caller::<EfinityEnvironment>(bob());
            let hero = game.create_hero();

            // verify the hero's tokens for weapon and armor were minted
            assert_eq!(hero, game.heroes.get(bob()).unwrap());

            MOCK_EFINITY.with(|efinity| {
                let efinity = efinity.borrow();

                // assert weapon token is frozen
                let weapon_token = efinity.token_of(collection_id, hero.weapon_id).unwrap();
                assert!(weapon_token.is_frozen);

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
            let mut game = init_game(initial_config.clone());
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
        fn test_start_battle() {
            let config = Config {
                enemy_health_range: (10, 20).into(),
                enemy_strength_range: (30, 50).into(),
                enemy_wearing_hat_chance: 100,
                ..Default::default()
            };
            let mut game = init_game(config.clone());

            // starting a battle without a hero fails
            assert_eq!(game.start_battle().unwrap_err(), Error::HeroNotFound);

            // create the hero and then start the battle
            game.create_hero();
            game.start_battle().unwrap();
            let hero = game.get_hero(alice()).unwrap();
            let enemy = hero.battle.unwrap().enemy;

            // enemy should be wearing a hat
            let hat_id = enemy.hat_id.unwrap();
            let metadata = game.get_metadata(hat_id).unwrap().unwrap();
            assert_eq!(metadata.token_type, TokenType::Hat);

            // the enemy stats should be in the correct ranges
            assert!(config.enemy_health_range.contains(enemy.health));
            assert!(config.enemy_strength_range.contains(enemy.strength));

            // lets change the config to never make an enemy wear a hat
            let mutation = ConfigMutation {
                enemy_wearing_hat_chance: Some(0),
                ..Default::default()
            };
            game.mutate_config(mutation).unwrap();

            // bob starts a battle
            test::set_caller::<EfinityEnvironment>(bob());
            game.create_hero();
            game.start_battle().unwrap();

            // ensure the enemy has no hat
            let hero = game.get_hero(bob()).unwrap();
            let enemy = hero.battle.unwrap().enemy;
            assert!(enemy.hat_id.is_none());
        }

        #[ink::test]
        fn test_advance_battle() {
            // give hero and enemy a lot of health so they don't die
            let config = Config {
                hero_max_health: 100,
                enemy_health_range: (100, 100).into(),
                ..Default::default()
            };
            let mut game = init_game(config);
            game.create_hero();
            game.start_battle().unwrap();
            let initial_enemy = game.get_hero(alice()).unwrap().battle.unwrap().enemy;

            // make sure attack works
            game.advance_battle(Command::Attack).unwrap();
            let hero = game.get_hero(alice()).unwrap();
            let hero_strength = game.get_metadata(hero.weapon_id).unwrap().unwrap().strength;
            let battle = hero.battle.unwrap();
            assert_eq!(
                hero.health,
                game.config.hero_max_health - initial_enemy.strength
            );
            assert_eq!(battle.enemy.health, initial_enemy.health - hero_strength);
            assert_eq!(battle.round_number, 1);

            // try to heal without potion fails
            assert_eq!(
                game.advance_battle(Command::Heal).unwrap_err(),
                Error::HeroHasNoPotions
            );

            // give the hero a potion
            let mut hero = game.get_hero(alice()).unwrap();
            let enemy_health = hero.battle.unwrap().enemy.health;
            hero.health = 50;
            hero.potion_count = 1;
            game.heroes.insert(alice(), &hero);

            // now healing works
            game.advance_battle(Command::Heal).unwrap();
            let hero = game.get_hero(alice()).unwrap();
            assert!(hero.health > 50);
            assert_eq!(hero.potion_count, 0);
            assert_eq!(hero.battle.unwrap().enemy.health, enemy_health);
        }

        #[ink::test]
        fn test_win_battle() {
            let config = Config {
                enemy_health_range: (1, 1).into(),
                enemy_wearing_hat_chance: 100,
                ..Default::default()
            };
            let mut game = init_game(config);
            let caller = bob();
            test::set_caller::<EfinityEnvironment>(caller);

            game.create_hero();
            game.start_battle().unwrap();

            // set hero health to 1 less than max health
            let mut hero = game.get_hero(caller).unwrap();
            hero.health = game.config.hero_max_health - 1;

            // verify the enemy's hat is owned by the contract
            let mut battle = hero.battle.unwrap();
            let hat_id = battle.enemy.hat_id.unwrap();
            // the contract should own the hat
            assert_eq!(
                game.env().extension().balance_of(
                    game.collection_id,
                    hat_id,
                    game.env().account_id()
                ),
                1
            );

            // defeat the enemy
            game.advance_battle(Command::Attack).unwrap();

            // make sure the correct amount of gold is received
            let gold_amount = game.get_gold_balance(caller);
            assert!(game.config.enemy_gold_drop_range.contains(gold_amount as _));

            // the hat should now be owned by the hero
            assert_ne!(game.env().account_id(), caller);
            assert_eq!(
                game.env()
                    .extension()
                    .balance_of(game.collection_id, hat_id, caller),
                1
            );
        }

        #[ink::test]
        fn test_lose_battle() {
            let mut game = init_game(Default::default());
            game.create_hero();
            game.start_battle().unwrap();

            // set hero health to 1 and increase victory count
            let mut hero = game.get_hero(alice()).unwrap();
            hero.health = 1;
            hero.consecutive_victory_count = 5;
            game.heroes.insert(alice(), &hero);

            // lose the battle
            game.advance_battle(Command::Attack).unwrap();
            let mut hero = game.get_hero(alice()).unwrap();
            assert!(hero.battle.is_none());

            // health and victory count should be reset
            assert_eq!(hero.health, game.config.hero_max_health);
            assert_eq!(hero.consecutive_victory_count, 0);
        }

        #[ink::test]
        fn test_set_hero_equipment() {
            let mut game = init_game(Default::default());
            let hero = game.create_hero();
            let initial_weapon_id = hero.weapon_id;

            // cannot set to token that does not exist
            assert_eq!(game.equip(10).unwrap_err(), Error::InvalidEquipment);

            // Mint a new token. It will still fail because there is no weapon attribute
            let new_weapon_id = game.mint_nft(alice(), false);
            assert_eq!(
                game.equip(new_weapon_id).unwrap_err(),
                Error::InvalidEquipment
            );

            // add equipment attribute and now it works
            game.add_equipment_attribute(new_weapon_id, TokenType::Weapon, Some((1, 1).into()));
            game.equip(new_weapon_id).unwrap();

            // make sure new weapon is frozen. Old one is not frozen.
            MOCK_EFINITY.with(|efinity| {
                let efinity = efinity.borrow();
                assert!(
                    !efinity
                        .token_of(game.collection_id, initial_weapon_id)
                        .unwrap()
                        .is_frozen
                );
                assert!(
                    efinity
                        .token_of(game.collection_id, new_weapon_id)
                        .unwrap()
                        .is_frozen
                );
            });

            let hero = game.heroes.get(alice()).unwrap();
            assert_eq!(hero.weapon_id, new_weapon_id);
        }

        #[ink::test]
        fn test_rest() {
            let config = Config {
                rest_cost: 10,
                ..Default::default()
            };
            let mut game = init_game(config.clone());

            // cannot rest without hero
            assert_eq!(game.rest(), Err(Error::HeroNotFound));

            // create hero with 1 health
            let mut hero = game.create_hero();
            hero.health = 1;
            game.heroes.insert(alice(), &hero);

            // cant rest if you don't have enough gold
            assert_eq!(game.rest(), Err(Error::NotEnoughGold));

            // mint gold and then can rest
            game.mint_gold(20);
            assert_eq!(game.get_gold_balance(alice()), 20);
            game.rest().unwrap();
            assert_eq!(game.get_gold_balance(alice()), 10);
            assert_eq!(
                game.get_hero(alice()).unwrap().health,
                config.hero_max_health
            );
        }

        #[test]
        fn test_buy_potion() {
            #[ink::test]
            fn test_rest() {
                let config = Config {
                    potion_cost: 10,
                    ..Default::default()
                };
                let mut game = init_game(config.clone());

                // cannot buy without hero
                assert_eq!(game.buy_potion(1), Err(Error::HeroNotFound));

                // cant buy if you don't have enough gold
                let mut hero = game.create_hero();
                game.mint_gold(15);
                assert_eq!(game.buy_potion(2), Err(Error::NotEnoughGold));

                // mint gold and then buy the potion
                game.mint_gold(5);
                game.buy_potion(2).unwrap();
                assert_eq!(game.get_gold_balance(alice()), 0);
                assert_eq!(game.get_hero(alice()).unwrap().potion_count, 2);
            }
        }

        #[ink::test]
        fn test_buy_weapon() {}

        #[test]
        fn test_lerp() {
            assert_eq!(lerp(0, 100, u32::MAX), 100);
            assert_eq!(lerp(0, 100, (u32::MAX / 2) + 1), 50);
            assert_eq!(lerp(0, 100, (u32::MAX / 10) + 1), 10);
            assert_eq!(lerp(5, 100, 0), 5);
        }

        #[test]
        fn test_range() {
            let range = Range { start: 5, end: 10 };

            // contains
            assert!(range.contains(5));
            assert!(range.contains(7));
            assert!(range.contains(10));

            // does not contain
            assert!(!range.contains(4));
            assert!(!range.contains(11));
        }
    }
}

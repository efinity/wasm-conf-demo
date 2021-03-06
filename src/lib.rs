//! A simple smart contract game for Efinity

#![cfg_attr(not(feature = "std"), no_std)]
// additional safety
#![deny(unused_must_use)]

#[cfg(test)]
mod mock;
mod types;

use efinity_contracts::{prelude::*, Freeze, FreezeType};
use ink::codegen::Env;
use ink_lang as ink;
use ink_storage::{traits::SpreadAllocate, Mapping};
use scale::{Decode, Encode};
use types::{Error, Result, *};

/// The attribute key used for equipment
fn attribute_key() -> AttributeKey {
    b"equipment".to_vec()
}

/// Multi-Tokens example smart contract
#[ink::contract(env = EfinityEnvironment)]
#[allow(clippy::extra_unused_lifetimes, clippy::let_unit_value)]
mod game {
    use super::*;

    /// A hero was created
    #[ink(event)]
    pub struct HeroCreated {
        /// The `AccountId` of the hero
        pub hero_id: AccountId,
        /// The token id of the weapon
        pub weapon_id: TokenId,
        /// The strength of the weapon
        pub weapon_strength: u32,
    }

    /// A battle was started
    #[ink(event)]
    pub struct BattleStarted {
        /// The `AccountId` of the hero
        pub hero_id: AccountId,
        /// The enemy generated for this battle
        pub enemy: Enemy,
    }

    /// The battle was advanced by a round
    #[ink(event)]
    pub struct BattleAdvanced {
        /// The `AccountId` of the hero
        pub hero_id: AccountId,
        /// The round number of the battle
        pub round_number: u32,
        /// The damage dealt to the hero
        pub hero_damage_received: u32,
        /// The damage dealt to the enemy
        pub enemy_damage_received: u32,
    }

    /// A battle ended
    #[ink(event)]
    pub struct BattleEnded {
        /// The `AccountId` of the hero
        pub hero_id: AccountId,
        /// True if the hero won the battle
        pub hero_wins: bool,
        /// The total number of rounds the battle took
        pub round_count: u32,
    }

    /// A weapon was purchased
    #[ink(event)]
    pub struct WeaponPurchased {
        /// The `AccountId` of the hero
        pub hero_id: AccountId,
        /// The `TokenId` of the weapon purchased
        pub token_id: TokenId,
        /// The strength of the weapon
        pub strength: u32,
    }

    /// A hero rested
    #[ink(event)]
    pub struct Rested {
        /// The `AccountId` of the hero
        pub hero_id: AccountId,
    }

    /// Equipment was changed for a hero
    #[ink(event)]
    pub struct EquipmentChanged {
        /// The `AccountId` of the hero
        pub hero_id: AccountId,
        /// The `TokenId` of the equipment
        pub token_id: TokenId,
        /// True if it was equipped, false if it was unequipped
        pub equipped: bool,
    }

    /// The storage for this contract
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Game {
        /// The configuration for the game
        config: Config,
        /// The owner of the contract
        owner: AccountId,
        /// The collection id that all tokens of the game use
        collection_id: CollectionId,
        /// The id of the token used as gold
        gold_token_id: TokenId,
        /// The id of the collection used for all tokens
        next_token_id: TokenId,
        /// The nonce used for randomness
        random_nonce: u32,
        /// The seed used for randomness
        random_seed: u32,
        /// A map of heroes by account id
        heroes: Mapping<AccountId, Hero>,
    }

    impl Game {
        /// Create a new game instance
        /// ### Parameters
        /// * `collection_id` - The collection id that all tokens will use
        /// * `gold_token_id` - The id of the token used as gold
        /// * `initial_token_id` - The first token id used for NFTs. This will be incremented for each token.
        /// * `random_seed` - A value used to differentiate randomness between games
        /// * `config` - The config used for the game. If not provided, it will use default values.
        ///
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

        /// Transfer ownership of the contract to `new_owner`
        #[ink(message)]
        pub fn transfer_contract_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            // make sure the owner is the caller
            if self.env().caller() != self.owner {
                return Err(Error::NoPermission);
            }

            // change the owner
            self.owner = new_owner;

            Ok(())
        }

        /// Modify the configuration of the game. Only callable by the owner.
        #[ink(message)]
        pub fn mutate_config(&mut self, mutation: ConfigMutation) -> Result<()> {
            // make sure the owner is the caller
            if self.env().caller() != self.owner {
                return Err(Error::NoPermission);
            }

            // apply the mutation
            mutation.apply_to(&mut self.config);

            Ok(())
        }

        /// Returns the game's config
        #[ink(message)]
        pub fn get_config(&self) -> Config {
            self.config.clone()
        }

        /// Returns the token type and id of a token id
        #[ink(message)]
        pub fn get_token_info(&self, token_id: TokenId) -> TokenIdInfo {
            let token_id = WrappedTokenId(token_id);
            TokenIdInfo {
                token_type: token_id.token_type(),
                id: token_id.id(),
            }
        }

        /// Create a hero for the caller
        #[ink(message)]
        pub fn create_hero(&mut self) -> Result<Hero> {
            let caller = self.env().caller();

            // mint the weapon token
            let weapon_id = self.mint_nft(caller, TokenType::Weapon, true)?;

            // add attribute to equipment tokens
            let weapon_strength = self
                .add_equipment_attribute(weapon_id, self.config.starting_weapon_strength_range)?;

            // create hero with the token we just minted
            let hero = Hero::new(
                self.config.hero_max_health,
                weapon_id,
                self.config.hero_initial_potion_count,
            );
            self.heroes.insert(caller, &hero);

            // emit the event
            self.env().emit_event(HeroCreated {
                hero_id: caller,
                weapon_id,
                weapon_strength,
            });
            Ok(hero)
        }

        /// Start a battle with a randomly generated enemy
        #[ink(message)]
        pub fn start_battle(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let mut hero = self.heroes.get(caller).ok_or(Error::HeroNotFound)?;

            // possibly generate a hat for the enemy
            let hat_id = {
                if self.random_chance(self.config.enemy_wearing_hat_chance) {
                    // the hat is owned by the contract
                    Some(self.mint_nft(self.env().account_id(), TokenType::Hat, false)?)
                } else {
                    None
                }
            };

            // create the enemy
            let enemy = Enemy {
                hat_id,
                health: self.random_in_range(self.config.enemy_health_range),
                strength: self.random_in_range(self.config.enemy_strength_range),
            };

            // update the data
            hero.battle = Some(Battle::new(enemy));
            self.heroes.insert(caller, &hero);

            // emit the event
            self.env().emit_event(BattleStarted {
                hero_id: caller,
                enemy,
            });

            Ok(())
        }

        /// Advance the battle to the next turn
        #[ink(message)]
        pub fn advance_battle(&mut self, command: Command) -> Result<()> {
            /// Returns true if the battle is over
            fn battle_is_over(hero: &Hero, battle: &Battle) -> bool {
                hero.is_dead() || battle.enemy.is_dead()
            }

            // setup
            let caller = self.env().caller();
            let mut hero = self.heroes.get(caller).ok_or(Error::HeroNotFound)?;
            let mut battle = hero.battle.ok_or(Error::HeroNotInBattle)?;
            let hero_initial_health = hero.health;
            let enemy_initial_health = battle.enemy.health;

            // perform actions
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

            // send the event
            self.env().emit_event(BattleAdvanced {
                hero_id: caller,
                round_number: battle.round_number,
                hero_damage_received: hero_initial_health.saturating_sub(hero.health),
                enemy_damage_received: enemy_initial_health.saturating_sub(battle.enemy.health),
            });
            battle.round_number = battle.round_number.saturating_add(1);

            // process battle outcome
            if battle_is_over(&hero, &battle) {
                hero.battle = None;

                // process hero victory
                if battle.enemy.is_dead() {
                    // update victory count
                    hero.consecutive_victory_count =
                        hero.consecutive_victory_count.saturating_add(1);
                    if hero.highest_consecutive_victory_count < hero.consecutive_victory_count {
                        hero.highest_consecutive_victory_count = hero.consecutive_victory_count;
                    }

                    // give gold reward
                    let gold_amount = self.random_in_range(self.config.enemy_gold_drop_range);
                    self.mint_gold(gold_amount as TokenBalance)?;

                    // transfer the hat to the hero if it exists
                    if let Some(hat_id) = battle.enemy.hat_id {
                        self.env().extension().transfer(
                            caller,
                            self.collection_id,
                            TransferParams::Simple {
                                token_id: hat_id,
                                amount: 1,
                                keep_alive: false,
                            },
                        )?;
                    }
                }

                // process hero loss
                if hero.is_dead() {
                    // update hero stats
                    hero.health = self.config.hero_max_health;
                    hero.consecutive_victory_count = 0;

                    // burn the enemy's hat if it won the battle with it
                    if let Some(hat_id) = battle.enemy.hat_id {
                        self.env().extension().burn(
                            self.collection_id,
                            BurnParams {
                                token_id: hat_id,
                                amount: 1,
                                keep_alive: false,
                                remove_token_storage: true,
                            },
                        )?;
                    }
                }

                // emit event
                self.env().emit_event(BattleEnded {
                    hero_id: caller,
                    hero_wins: !hero.is_dead(),
                    round_count: battle.round_number,
                });
            } else {
                hero.battle = Some(battle);
            }

            // update the data
            self.heroes.insert(caller, &hero);

            Ok(())
        }

        /// Returns the `Hero` for `account_id` if it exists
        #[ink(message)]
        pub fn get_hero(&self, account_id: AccountId) -> Option<Hero> {
            self.heroes.get(account_id)
        }

        /// Equip `token_id` for the caller
        #[ink(message)]
        pub fn equip(&mut self, token_id: TokenId) -> Result<()> {
            let caller = self.env().caller();
            let mut hero = self.heroes.get(caller).ok_or(Error::HeroNotFound)?;

            // get the token type
            let token_type = WrappedTokenId(token_id)
                .token_type()
                .ok_or(Error::InvalidEquipment)?;

            // Make sure if it's a weapon, it has metadata
            if matches!(token_type, TokenType::Weapon) && self.get_metadata(token_id)?.is_none() {
                return Err(Error::InvalidEquipment);
            }

            // set equipment and prepare thaw
            let thaw_token_id: Option<TokenId>;
            match token_type {
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
                })?;
            }

            // freeze new equipped token
            self.env().extension().freeze(Freeze {
                collection_id: self.collection_id,
                freeze_type: FreezeType::Token(token_id),
            })?;

            // update the hero
            self.heroes.insert(caller, &hero);

            // emit event
            self.env().emit_event(EquipmentChanged {
                hero_id: caller,
                token_id,
                equipped: true,
            });

            Ok(())
        }

        /// Remove the caller's hat
        #[ink(message)]
        pub fn unequip_hat(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let mut hero = self
                .get_hero(self.env().caller())
                .ok_or(Error::HeroNotFound)?;

            // remove the hat
            if let Some(hat_id) = hero.hat_id {
                hero.hat_id = None;
                self.heroes.insert(caller, &hero);

                // thaw the hat
                self.env().extension().thaw(Freeze {
                    collection_id: self.collection_id,
                    freeze_type: FreezeType::Token(hat_id),
                })?;

                // emit event
                self.env().emit_event(EquipmentChanged {
                    hero_id: caller,
                    token_id: hat_id,
                    equipped: false,
                });
            }

            Ok(())
        }

        /// Returns the `TokenMetadata` for `token_id` if it exists
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

        /// Purchase a healing potion. Can only be done outside of battle.
        #[ink(message)]
        pub fn buy_potion(&mut self, quantity: u32) -> Result<()> {
            let mut hero =
                self.spend_gold(self.config.potion_cost.saturating_mul(quantity as _))?;

            // add the potions
            hero.potion_count = hero.potion_count.saturating_add(quantity);
            self.heroes.insert(self.env().caller(), &hero);

            Ok(())
        }

        /// Buy a new weapon. Can only be done outside of battle.
        /// Returns the `TokenId` of the generated weapon.
        #[ink(message)]
        pub fn buy_weapon(&mut self) -> Result<TokenId> {
            let caller = self.env().caller();
            self.spend_gold(self.config.weapon_cost)?;

            // generate the weapon
            let token_id = self.mint_nft(caller, TokenType::Weapon, false)?;
            let strength = self
                .add_equipment_attribute(token_id, self.config.purchased_weapon_strength_range)?;
            self.env().emit_event(WeaponPurchased {
                hero_id: caller,
                token_id,
                strength,
            });

            Ok(token_id)
        }

        /// Recover the caller to full health. Can only be done outside of battle.
        #[ink(message)]
        pub fn rest(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let mut hero = self.spend_gold(self.config.rest_cost)?;

            // set health to max
            hero.health = self.config.hero_max_health;
            self.heroes.insert(self.env().caller(), &hero);

            // emit event
            self.env().emit_event(Rested { hero_id: caller });

            Ok(())
        }

        /// Returns the balance of gold for `account_id`
        #[ink(message)]
        pub fn get_gold_balance(&self, account_id: AccountId) -> TokenBalance {
            self.env()
                .extension()
                .balance_of(self.collection_id, self.gold_token_id, account_id)
        }
    }

    // helper functions
    impl Game {
        /// Returns the current token id and increments `next_token_id`
        fn increment_next_token_id(&mut self) -> TokenId {
            let token_id = self.next_token_id;
            self.next_token_id += 1;
            token_id
        }

        /// Mints a non-fungible token to `recipient`. Freezes it if `freeze` is true.
        fn mint_nft(
            &mut self,
            recipient: AccountId,
            token_type: TokenType,
            freeze: bool,
        ) -> Result<TokenId> {
            let id = self.increment_next_token_id();
            let token_id = WrappedTokenId::new(id, Some(token_type)).0;

            let params = MintParams::CreateToken {
                token_id,
                initial_supply: 1,
                unit_price: self.env().extension().get_token_account_deposit(),
                cap: Some(TokenCap::SingleMint),
                royalty: None,
            };
            self.env()
                .extension()
                .mint(recipient, self.collection_id, params)?;
            if freeze {
                self.env().extension().freeze(Freeze {
                    collection_id: self.collection_id,
                    freeze_type: FreezeType::Token(token_id),
                })?;
            }
            Ok(token_id)
        }

        /// Mints `amount` gold to the caller
        fn mint_gold(&mut self, amount: TokenBalance) -> Result<()> {
            let params = MintParams::Mint {
                token_id: self.gold_token_id,
                amount,
                unit_price: None,
            };
            self.env()
                .extension()
                .mint(self.env().caller(), self.collection_id, params)?;
            Ok(())
        }

        /// Adds `TokenMetadata` to `token_id` and sets `token_type`. Generates strength value if `strength_range` is
        /// Some. Returns the generated strength.
        fn add_equipment_attribute(
            &mut self,
            token_id: TokenId,
            strength_range: Range,
        ) -> Result<u32> {
            let strength = self.random_in_range(strength_range);
            let metadata = TokenMetadata { strength };
            self.env().extension().set_attribute(
                self.collection_id,
                Some(token_id),
                attribute_key(),
                metadata.encode(),
            )?;
            Ok(strength)
        }

        /// Spends `amount` gold from the caller's account. Returns the hero.
        fn spend_gold(&mut self, amount: TokenBalance) -> Result<Hero> {
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
            if gold_balance < amount {
                return Err(Error::NotEnoughGold);
            }

            // transfer gold to the contract
            self.env().extension().transfer(
                self.env().account_id(),
                self.collection_id,
                TransferParams::Operator {
                    token_id: self.gold_token_id,
                    source: self.env().caller(),
                    amount,
                    keep_alive: true,
                },
            )?;

            // burn the token units
            let params = BurnParams {
                token_id: self.gold_token_id,
                amount,
                keep_alive: false,
                remove_token_storage: false,
            };
            self.env().extension().burn(self.collection_id, params)?;

            Ok(hero)
        }

        /// Handles `hero`'s action in `battle`, according to `command`.
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
                    let attack_power = self.calculate_attack_power(metadata.strength);
                    battle.enemy.health = battle.enemy.health.saturating_sub(attack_power);
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

        /// Handles enemy's action in `battle` with `hero`.
        fn enemy_action(&mut self, hero: &mut Hero, battle: &mut Battle) -> Result<()> {
            let enemy = &mut battle.enemy;
            let attack_power = self.calculate_attack_power(enemy.strength);
            hero.health = hero.health.saturating_sub(attack_power);
            Ok(())
        }

        /// Computes a random number in `range`
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

        /// Generates a random number between 1 and 100. Returns true if this number is less than chance.
        fn random_chance(&mut self, chance: u32) -> bool {
            self.random_in_range((0, 100).into()) <= chance
        }

        /// Calculates attack power for strength, taking into account the config's attack variance.
        fn calculate_attack_power(&mut self, strength: u32) -> u32 {
            // this is a workaround because random_in_range supports unsigned only
            let unsigned_variance =
                self.random_in_range((0, self.config.attack_variance * 2 + 1).into());
            let delta = unsigned_variance as i32 - self.config.attack_variance as i32;
            (strength as i32 + delta) as u32
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
        output as u32
    }

    #[cfg(test)]
    pub mod tests {
        use super::*;
        use efinity_contracts::{mock, AccountId};
        use ink_env::test;
        use std::cell::RefCell;

        /// Set up the game for tests
        fn init_game(config: Config) -> Game {
            // instantiate the smart contract
            let mut game = Game::new(1000, 0, 1, 0, Some(config));

            // for tests to work, the mock chain extension needs to be registered
            mock::register_chain_extension(game.env().account_id());

            // create the collection used by the game
            game.collection_id = game
                .env()
                .extension()
                .create_collection(Default::default())
                .unwrap();

            // create the token used as gold
            let params = MintParams::CreateToken {
                token_id: game.gold_token_id,
                initial_supply: 1,
                unit_price: game.env().extension().get_token_account_deposit(),
                cap: Some(TokenCap::SingleMint),
                royalty: None,
            };
            game.env()
                .extension()
                // mint to frank because we don't use that account in other tests
                .mint(accounts().frank, game.collection_id, params)
                .unwrap();

            game
        }

        /// Helper function to get default accounts
        fn accounts() -> test::DefaultAccounts<EfinityEnvironment> {
            test::default_accounts()
        }

        /// Alice's account id
        fn alice() -> AccountId {
            accounts().alice
        }

        /// Bob's account id
        fn bob() -> AccountId {
            accounts().bob
        }

        /// Test `create_hero` function
        #[ink::test]
        fn test_create_hero() {
            // init game
            let config = Config {
                hero_initial_potion_count: 5,
                ..Default::default()
            };
            let mut game = init_game(config.clone());
            let collection_id = game.collection_id;

            // create a hero for bob
            test::set_caller::<EfinityEnvironment>(bob());
            let hero = game.create_hero().unwrap();
            assert_eq!(hero.health, config.hero_max_health);
            assert_eq!(hero.potion_count, config.hero_initial_potion_count);

            // verify the hero's tokens for weapon and armor were minted
            assert_eq!(hero, game.heroes.get(bob()).unwrap());

            // assert weapon token is frozen
            let weapon_token = game
                .env()
                .extension()
                .token_of(collection_id, hero.weapon_id)
                .unwrap();
            assert!(weapon_token.is_frozen);

            // assert bob has the weapon token
            assert_eq!(
                game.env()
                    .extension()
                    .balance_of(collection_id, hero.weapon_id, bob()),
                1
            );

            // assert the weapon token attribute exists
            assert!(game
                .env()
                .extension()
                .attribute_of(collection_id, Some(hero.weapon_id), attribute_key())
                .is_some());
        }

        /// Test `mutate_config` function
        #[ink::test]
        fn test_mutate_config() {
            let initial_config = Config::default();
            let mut game = init_game(initial_config.clone());
            let mutation = ConfigMutation {
                potion_cost: Some(1000),
                ..Default::default()
            };
            game.mutate_config(mutation.clone()).unwrap();
            let mut config = game.get_config();

            // make sure the potion cost changed
            assert_eq!(config.potion_cost, 1000);

            // if we change the potion cost back, everything else should be the same
            config.potion_cost = initial_config.potion_cost;
            assert_eq!(initial_config, config);

            // bob cannot change the config
            test::set_caller::<EfinityEnvironment>(bob());
            assert_eq!(game.mutate_config(mutation), Err(Error::NoPermission));
        }

        /// Test `transfer_contract_ownership` function
        #[ink::test]
        fn test_transfer_contract_ownership() {
            let initial_config = Config::default();
            let mut game = init_game(initial_config.clone());

            // alice is the owner
            assert_eq!(game.owner, alice());

            // alice transfers to bob. Now he's the owner.
            game.transfer_contract_ownership(bob()).unwrap();
            assert_eq!(game.owner, bob());

            // alice cannot transfer ownership any more
            assert_eq!(
                game.transfer_contract_ownership(alice()),
                Err(Error::NoPermission)
            );
        }

        /// Test `start_battle` function
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
            game.create_hero().unwrap();
            game.start_battle().unwrap();
            let hero = game.get_hero(alice()).unwrap();
            let enemy = hero.battle.unwrap().enemy;

            // enemy should be wearing a hat
            let hat_id = WrappedTokenId(enemy.hat_id.unwrap());
            assert_eq!(hat_id.token_type(), Some(TokenType::Hat));

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
            game.create_hero().unwrap();
            game.start_battle().unwrap();

            // ensure the enemy has no hat
            let hero = game.get_hero(bob()).unwrap();
            let enemy = hero.battle.unwrap().enemy;
            assert!(enemy.hat_id.is_none());
        }

        /// Test `advance_battle` function
        #[ink::test]
        fn test_advance_battle() {
            // give hero and enemy a lot of health so they don't die
            let config = Config {
                hero_initial_potion_count: 0,
                hero_max_health: 100,
                enemy_health_range: (100, 100).into(),
                ..Default::default()
            };
            let attack_variance = config.attack_variance;
            let mut game = init_game(config);
            game.create_hero().unwrap();
            game.start_battle().unwrap();
            let initial_enemy = game.get_hero(alice()).unwrap().battle.unwrap().enemy;

            // make sure attack works
            game.advance_battle(Command::Attack).unwrap();
            let hero = game.get_hero(alice()).unwrap();
            let hero_strength = game.get_metadata(hero.weapon_id).unwrap().unwrap().strength;
            let battle = hero.battle.unwrap();

            // check hero health
            let expected_hero_health = Range::new(
                game.config.hero_max_health - initial_enemy.strength - attack_variance,
                game.config.hero_max_health - initial_enemy.strength + attack_variance,
            );
            assert!(expected_hero_health.contains(hero.health));

            // check enemy health
            let expected_enemy_health = Range::new(
                initial_enemy.health - hero_strength - attack_variance,
                initial_enemy.health - hero_strength + attack_variance,
            );
            assert!(expected_enemy_health.contains(battle.enemy.health));
            assert_eq!(battle.round_number, 1);

            // trying to heal without potion fails
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

        /// Test `advance_battle` where the hero wins
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

            game.create_hero().unwrap();
            game.start_battle().unwrap();

            // set hero health to 1 less than max health
            let mut hero = game.get_hero(caller).unwrap();
            hero.health = game.config.hero_max_health - 1;

            // verify the enemy's hat is owned by the contract
            let battle = hero.battle.unwrap();
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

            let hero = game.get_hero(caller).unwrap();
            assert_eq!(hero.consecutive_victory_count, 1);
            assert_eq!(hero.highest_consecutive_victory_count, 1);

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

        /// Test `advance_battle` where the hero loses
        #[ink::test]
        fn test_lose_battle() {
            let mut game = init_game(Config {
                enemy_wearing_hat_chance: 100,
                ..Default::default()
            });
            game.create_hero().unwrap();
            game.start_battle().unwrap();

            // set hero health to 1 and increase victory count
            let mut hero = game.get_hero(alice()).unwrap();
            hero.health = 1;
            hero.consecutive_victory_count = 5;
            game.heroes.insert(alice(), &hero);

            // the hat token exists
            let hat_id = hero.battle.unwrap().enemy.hat_id.unwrap();
            assert_eq!(
                game.env().extension().balance_of(
                    game.collection_id,
                    hat_id,
                    game.env().account_id()
                ),
                1
            );

            // lose the battle
            game.advance_battle(Command::Attack).unwrap();
            let hero = game.get_hero(alice()).unwrap();
            assert!(hero.battle.is_none());

            // health and victory count should be reset
            assert_eq!(hero.health, game.config.hero_max_health);
            assert_eq!(hero.consecutive_victory_count, 0);

            // the hat token was burned
            assert_eq!(
                game.env().extension().balance_of(
                    game.collection_id,
                    hat_id,
                    game.env().account_id()
                ),
                0
            );
        }

        /// Test `equip` function
        #[ink::test]
        fn test_equip() {
            let mut game = init_game(Default::default());
            let hero = game.create_hero().unwrap();
            let initial_weapon_id = hero.weapon_id;

            // cannot set to token that does not exist
            assert_eq!(game.equip(10).unwrap_err(), Error::InvalidEquipment);

            // Mint a new token. It will still fail because there is no weapon attribute
            let new_weapon_id = game.mint_nft(alice(), TokenType::Weapon, false).unwrap();
            assert_eq!(
                game.equip(new_weapon_id).unwrap_err(),
                Error::InvalidEquipment
            );

            // add equipment attribute and now it works
            game.add_equipment_attribute(new_weapon_id, (1, 1).into())
                .unwrap();
            game.equip(new_weapon_id).unwrap();

            // make sure new weapon is frozen. Old one is not frozen.
            assert!(
                !game
                    .env()
                    .extension()
                    .token_of(game.collection_id, initial_weapon_id)
                    .unwrap()
                    .is_frozen
            );
            assert!(
                game.env()
                    .extension()
                    .token_of(game.collection_id, new_weapon_id)
                    .unwrap()
                    .is_frozen
            );

            let hero = game.heroes.get(alice()).unwrap();
            assert_eq!(hero.weapon_id, new_weapon_id);
        }

        /// Test `unequip_hat` function
        #[ink::test]
        fn test_unequip_hat() {
            let mut game = init_game(Default::default());
            game.create_hero().unwrap();

            // equip a hat
            let hat_id = game.mint_nft(alice(), TokenType::Hat, false).unwrap();
            game.equip(hat_id).unwrap();

            // hero should be wearing the hat
            let hero = game.heroes.get(alice()).unwrap();
            assert_eq!(hero.hat_id.unwrap(), hat_id);

            // unequip the hat
            game.unequip_hat().unwrap();

            // hero should not be wearing it
            let hero = game.heroes.get(alice()).unwrap();
            assert!(hero.hat_id.is_none());

            // make sure the hat is not frozen
            assert!(
                !game
                    .env()
                    .extension()
                    .token_of(game.collection_id, hat_id)
                    .unwrap()
                    .is_frozen
            );
        }

        /// Test `rest` function
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
            let mut hero = game.create_hero().unwrap();
            hero.health = 1;
            game.heroes.insert(alice(), &hero);

            // cant rest if you don't have enough gold
            assert_eq!(game.rest(), Err(Error::NotEnoughGold));

            // mint gold and then can rest
            game.mint_gold(20).unwrap();
            assert_eq!(game.get_gold_balance(alice()), 20);
            game.rest().unwrap();
            assert_eq!(game.get_gold_balance(alice()), 10);
            assert_eq!(
                game.get_hero(alice()).unwrap().health,
                config.hero_max_health
            );
        }

        /// Test `buy_potion` function
        #[ink::test]
        fn test_buy_potion() {
            let config = Config {
                potion_cost: 10,
                hero_initial_potion_count: 0,
                ..Default::default()
            };
            let mut game = init_game(config);

            // cannot buy without hero
            assert_eq!(game.buy_potion(1), Err(Error::HeroNotFound));

            // cant buy if you don't have enough gold
            game.create_hero().unwrap();
            game.mint_gold(15).unwrap();
            assert_eq!(game.buy_potion(2), Err(Error::NotEnoughGold));

            // mint gold and then buy the potion
            game.mint_gold(5).unwrap();
            assert_eq!(game.get_gold_balance(alice()), 20);
            game.buy_potion(2).unwrap();
            assert_eq!(game.get_gold_balance(alice()), 0);
            assert_eq!(game.get_hero(alice()).unwrap().potion_count, 2);
        }

        /// Test `buy_weapon` function
        #[ink::test]
        fn test_buy_weapon() {
            let config = Config {
                weapon_cost: 10,
                purchased_weapon_strength_range: (50, 100).into(),
                ..Default::default()
            };
            let mut game = init_game(config.clone());

            // cannot buy without hero
            assert_eq!(game.buy_weapon(), Err(Error::HeroNotFound));

            // cant buy if you don't have enough gold
            game.create_hero().unwrap();
            assert_eq!(game.buy_weapon(), Err(Error::NotEnoughGold));

            // can now buy the weapon
            game.mint_gold(10).unwrap();
            let weapon_id = game.buy_weapon().unwrap();
            let metadata = game.get_metadata(weapon_id).unwrap().unwrap();

            // its strength should match the config
            assert!(config
                .purchased_weapon_strength_range
                .contains(metadata.strength));
        }

        /// Test `calculate_attack_power` function
        #[ink::test]
        fn test_calculate_attack_power() {
            fn new_game_with_attack_variance(attack_variance: u32) -> Game {
                init_game(Config {
                    attack_variance,
                    ..Default::default()
                })
            }

            // verify several cases of variance 2
            let mut game = new_game_with_attack_variance(2);
            for _ in 0..10 {
                assert!(Range::new(8, 12).contains(game.calculate_attack_power(10)));
            }

            // verify several cases of variance 5
            let mut game = new_game_with_attack_variance(5);
            for _ in 0..10 {
                assert!(Range::new(5, 15).contains(game.calculate_attack_power(10)));
            }

            // verify several cases of variance 0
            let mut game = new_game_with_attack_variance(0);
            for _ in 0..10 {
                assert_eq!(game.calculate_attack_power(10), 10);
            }
        }

        /// Test `lerp` function
        #[test]
        fn test_lerp() {
            assert_eq!(lerp(0, 100, u32::MAX), 100);
            assert_eq!(lerp(0, 100, (u32::MAX / 2) + 1), 50);
            assert_eq!(lerp(0, 100, (u32::MAX / 10) + 1), 10);
            assert_eq!(lerp(5, 100, 0), 5);
        }

        /// Test `Range` type
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

        /// Test `WrappedTokenId` type
        #[test]
        fn test_wrapped_token_id() {
            use TokenType::*;

            let token_id = 921324;

            let mut wrapped = WrappedTokenId(token_id);
            assert!(wrapped.token_type().is_none());

            wrapped.set_token_type(Some(Weapon));
            assert_eq!(wrapped.token_type(), Some(Weapon));
            assert_eq!(wrapped.id(), token_id);

            wrapped.set_token_type(Some(Hat));
            assert_eq!(wrapped.token_type(), Some(Hat));
            assert_eq!(wrapped.id(), token_id);

            wrapped.set_token_type(None);
            assert!(wrapped.token_type().is_none());
            assert_eq!(wrapped.id(), token_id);
        }
    }
}

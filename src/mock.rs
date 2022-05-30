use crate::{game::tests, AttributeKey, AttributeValue};
use efinity_contracts::{
    AccountId, Attribute, Balance, BurnParams, CollectionId, Freeze, FreezeType, MintParams,
    TokenBalance, TokenId, TransferParams,
};
use ink_env::test;
use scale::{Decode, Encode};
use std::collections::HashMap;
use tests::MOCK_EFINITY;

const MINT: u32 = 1140261079;
const TRANSFER: u32 = 3795401762;
const BURN: u32 = 532649603;
const GET_TOKEN_ACCOUNT_DEPOSIT: u32 = 299862019;
const SET_ATTRIBUTE: u32 = 2427127331;
const ATTRIBUTE_OF: u32 = 3842143254;
const BALANCE_OF: u32 = 1627189794;
const FREEZE: u32 = 1663653968;
const THAW: u32 = 885419348;

pub fn register_chain_extension() {
    test::register_chain_extension(MockExtensionFunction::<MINT>);
    test::register_chain_extension(MockExtensionFunction::<TRANSFER>);
    test::register_chain_extension(MockExtensionFunction::<SET_ATTRIBUTE>);
    test::register_chain_extension(MockExtensionFunction::<GET_TOKEN_ACCOUNT_DEPOSIT>);
    test::register_chain_extension(MockExtensionFunction::<ATTRIBUTE_OF>);
    test::register_chain_extension(MockExtensionFunction::<BALANCE_OF>);
    test::register_chain_extension(MockExtensionFunction::<BURN>);
    test::register_chain_extension(MockExtensionFunction::<FREEZE>);
    test::register_chain_extension(MockExtensionFunction::<THAW>);
}

#[derive(Default)]
pub struct MockChainExtension {
    /// This is a temporary workaround because there doesn't seem to be any way to read the caller
    /// of the chain extension
    pub contract_address: AccountId,
    pub attributes: HashMap<(CollectionId, Option<TokenId>, AttributeKey), Attribute>,
    pub tokens: HashMap<(CollectionId, TokenId), Token>,
    pub token_accounts: HashMap<(AccountId, CollectionId, TokenId), TokenAccount>,
}

impl MockChainExtension {
    fn call(&mut self, function_id: u32, input: &[u8], output: &mut Vec<u8>) -> u32 {
        match function_id {
            MINT => {
                // not sure why I have to start at index 1 instead of 0?
                let (recipient, collection_id, params): (AccountId, CollectionId, MintParams) =
                    Decode::decode(&mut &input[1..]).unwrap();
                match params {
                    MintParams::CreateToken {
                        token_id,
                        initial_supply,
                        ..
                    } => {
                        self.tokens.insert(
                            (collection_id, token_id),
                            Token {
                                supply: initial_supply,
                                is_frozen: false,
                            },
                        );
                        self.token_accounts.insert(
                            (recipient, collection_id, token_id),
                            TokenAccount {
                                balance: initial_supply,
                            },
                        );
                    }
                    MintParams::Mint {
                        token_id, amount, ..
                    } => {
                        self.tokens
                            .entry((collection_id, token_id))
                            .and_modify(|x| x.supply += amount)
                            .or_insert(Token {
                                supply: amount,
                                is_frozen: false,
                            });
                        self.token_accounts
                            .entry((recipient, collection_id, token_id))
                            .and_modify(|x| x.balance += amount)
                            .or_insert(TokenAccount { balance: amount });
                    }
                };
            }
            BURN => {
                let (collection_id, params): (CollectionId, BurnParams) =
                    Decode::decode(&mut &input[1..]).unwrap();
                let token_account = self
                    .token_accounts
                    .get_mut(&(self.contract_address, collection_id, params.token_id))
                    .expect("token account not found");
                token_account.balance = token_account.balance.saturating_sub(params.amount);

                let token = self
                    .tokens
                    .get_mut(&(collection_id, params.token_id))
                    .expect("token not found");
                token.supply = token.supply.saturating_sub(params.amount);
            }
            TRANSFER => {
                // I have no idea why this one requires different index in different circumstances
                let (target, collection_id, params): (AccountId, CollectionId, TransferParams) = {
                    if let Ok(value) = Decode::decode(&mut &input[1..]) {
                        value
                    } else {
                        Decode::decode(&mut &input[2..]).unwrap()
                    }
                };

                let (token_id, source, amount) = match params {
                    TransferParams::Simple {
                        token_id, amount, ..
                    } => (token_id, self.contract_address, amount),
                    TransferParams::Operator {
                        token_id,
                        source,
                        amount,
                        ..
                    } => (token_id, source, amount),
                };
                {
                    let source_account = self
                        .token_accounts
                        .get_mut(&(source, collection_id, token_id))
                        .unwrap();
                    source_account.balance = source_account.balance.saturating_sub(amount);
                }
                self.token_accounts
                    .entry((target, collection_id, token_id))
                    .and_modify(|x| x.balance = x.balance.saturating_add(amount))
                    .or_insert(TokenAccount { balance: amount });
            }
            FREEZE => {
                let freeze: Freeze = Decode::decode(&mut &input[1..]).unwrap();
                match freeze.freeze_type {
                    FreezeType::Token(token_id) => {
                        let token = self
                            .tokens
                            .get_mut(&(freeze.collection_id, token_id))
                            .expect("token not found");
                        token.is_frozen = true;
                    }
                    _ => unimplemented!(),
                }
            }
            THAW => {
                let freeze: Freeze = Decode::decode(&mut &input[1..]).unwrap();
                match freeze.freeze_type {
                    FreezeType::Token(token_id) => {
                        let token = self
                            .tokens
                            .get_mut(&(freeze.collection_id, token_id))
                            .expect("token not found");
                        token.is_frozen = false;
                    }
                    _ => unimplemented!(),
                }
            }
            SET_ATTRIBUTE => {
                let (collection_id, token_id, key, value): (
                    CollectionId,
                    Option<TokenId>,
                    AttributeKey,
                    AttributeValue,
                ) = Decode::decode(&mut &input[1..]).unwrap();
                self.attributes.insert(
                    (collection_id, token_id, key),
                    Attribute { value, deposit: 0 },
                );
            }
            GET_TOKEN_ACCOUNT_DEPOSIT => {
                let value: Balance = 100_000_000_000_000_000;
                Encode::encode_to(&value, output);
            }
            ATTRIBUTE_OF => {
                let (collection_id, token_id, key): (CollectionId, Option<TokenId>, AttributeKey) =
                    Decode::decode(&mut &input[1..]).unwrap();
                let attribute = self.attribute_of(collection_id, token_id, key);
                Encode::encode_to(&attribute, output);
            }
            BALANCE_OF => {
                // I don't know why this one must start at 2
                let (collection_id, token_id, account_id): (CollectionId, TokenId, AccountId) =
                    Decode::decode(&mut &input[2..]).unwrap();
                let balance = self.balance_of(collection_id, token_id, account_id);
                Encode::encode_to(&balance, output);
            }
            _ => panic!(),
        }
        0
    }

    pub fn token_of(&self, collection_id: CollectionId, token_id: TokenId) -> Option<&Token> {
        self.tokens.get(&(collection_id, token_id))
    }

    pub fn attribute_of(
        &self,
        collection_id: CollectionId,
        token_id: Option<TokenId>,
        key: AttributeKey,
    ) -> Option<&Attribute> {
        self.attributes.get(&(collection_id, token_id, key))
    }

    pub fn token_account_of(
        &self,
        collection_id: CollectionId,
        token_id: TokenId,
        account_id: AccountId,
    ) -> Option<&TokenAccount> {
        self.token_accounts
            .get(&(account_id, collection_id, token_id))
    }

    pub fn balance_of(
        &self,
        collection_id: CollectionId,
        token_id: TokenId,
        account_id: AccountId,
    ) -> TokenBalance {
        self.token_account_of(collection_id, token_id, account_id)
            .map(|x| x.balance)
            .unwrap_or_default()
    }
}

struct MockExtensionFunction<const FUNCTION_ID: u32>;

impl<const FUNCTION_ID: u32> test::ChainExtension for MockExtensionFunction<FUNCTION_ID> {
    fn func_id(&self) -> u32 {
        FUNCTION_ID
    }

    fn call(&mut self, input: &[u8], output: &mut Vec<u8>) -> u32 {
        MOCK_EFINITY.with(|x| x.borrow_mut().call(FUNCTION_ID, input, output))
    }
}

pub struct Token {
    pub supply: Balance,
    pub is_frozen: bool,
}

#[derive(Debug)]
pub struct TokenAccount {
    pub balance: Balance,
}

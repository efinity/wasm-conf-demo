use crate::{game::tests, AttributeKey, AttributeValue};
use efinity_contracts::{
    AccountId, Attribute, Balance, CollectionId, MintParams, TokenBalance, TokenId,
};
use ink_env::test;
use scale::{Decode, Encode};
use std::{borrow::Borrow, cell::RefCell, collections::HashMap};
use tests::MOCK_EFINITY;

const MINT: u32 = 1140261079;
const GET_TOKEN_ACCOUNT_DEPOSIT: u32 = 299862019;
const SET_ATTRIBUTE: u32 = 2427127331;
const ATTRIBUTE_OF: u32 = 3842143254;

pub fn register_chain_extension() {
    test::register_chain_extension(MockExtensionFunction::<MINT>);
    test::register_chain_extension(MockExtensionFunction::<SET_ATTRIBUTE>);
    test::register_chain_extension(MockExtensionFunction::<ATTRIBUTE_OF>);
    test::register_chain_extension(MockExtensionFunction::<GET_TOKEN_ACCOUNT_DEPOSIT>);
}

#[derive(Default)]
pub struct MockChainExtension {
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
                        unit_price,
                        cap,
                    } => {
                        self.tokens.insert(
                            (collection_id, token_id),
                            Token {
                                supply: initial_supply,
                            },
                        );
                        self.token_accounts.insert(
                            (recipient, collection_id, token_id),
                            TokenAccount {
                                balance: initial_supply,
                            },
                        );
                    }
                    MintParams::Mint { .. } => unimplemented!(),
                };
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
}

#[derive(Debug)]
pub struct TokenAccount {
    pub balance: Balance,
}

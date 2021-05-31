use crate::tests::{AccountState::*, PlasmaTestBuilder};
use num::{BigUint, Zero};
use zksync_crypto::PrivateKey;
use zksync_types::{Account, AccountId, AccountUpdate, Order, Swap, TokenId};

type TestAccount = (AccountId, Account, PrivateKey);

struct TestSwap {
    accounts: (usize, usize),
    recipients: (usize, usize),
    submitter: usize,
    tokens: (u32, u32),
    amounts: (u64, u64),
    balances: (u64, u64, u64),
    first_price: (u64, u64),
    second_price: (u64, u64),
    fee_token: u32,
    fee: u64,
    is_limit_order: (bool, bool),
    test_accounts: Vec<TestAccount>,
}

enum TestResult {
    Success(Vec<(u64, u64)>),
    Failure(&'static str),
}

use TestResult::*;

impl TestSwap {
    fn test_success(&self, mut tb: PlasmaTestBuilder, outcome: TestResult) {
        let (account_0_id, account_0, account_0_sk) = &self.test_accounts[self.accounts.0];
        let (account_1_id, account_1, account_1_sk) = &self.test_accounts[self.accounts.1];
        let (recipient_0_id, recipient_0, _) = &self.test_accounts[self.recipients.0];
        let (recipient_1_id, recipient_1, _) = &self.test_accounts[self.recipients.1];
        let (submitter_id, submitter, submitter_sk) = &self.test_accounts[self.submitter];

        let token_0 = TokenId(self.tokens.0);
        let token_1 = TokenId(self.tokens.1);
        let fee_token = TokenId(self.fee_token);
        let fee = BigUint::from(self.fee);

        let amount_0 = if self.is_limit_order.0 {
            BigUint::zero()
        } else {
            BigUint::from(self.amounts.0)
        };

        let amount_1 = if self.is_limit_order.1 {
            BigUint::zero()
        } else {
            BigUint::from(self.amounts.1)
        };

        let balances = (
            BigUint::from(self.balances.0),
            BigUint::from(self.balances.1),
            BigUint::from(self.balances.2),
        );

        tb.set_balance(*account_0_id, token_0, balances.0.clone());
        tb.set_balance(*account_1_id, token_1, balances.1.clone());
        tb.set_balance(*submitter_id, fee_token, balances.2);

        let order_0 = Order::new_signed(
            *account_0_id,
            recipient_0.address,
            account_0.nonce,
            token_0,
            token_1,
            (
                BigUint::from(self.first_price.0),
                BigUint::from(self.first_price.1),
            ),
            amount_0,
            Default::default(),
            &&account_0_sk,
        )
        .expect("order creation failed");

        let order_1 = Order::new_signed(
            *account_1_id,
            recipient_1.address,
            account_1.nonce,
            token_1,
            token_0,
            (
                BigUint::from(self.second_price.0),
                BigUint::from(self.second_price.1),
            ),
            amount_1,
            Default::default(),
            &account_1_sk,
        )
        .expect("order creation failed");

        let swap = Swap::new_signed(
            *submitter_id,
            submitter.address,
            submitter.nonce,
            (order_0, order_1),
            (BigUint::from(self.amounts.0), BigUint::from(self.amounts.1)),
            fee,
            fee_token,
            &submitter_sk,
        )
        .expect("swap creation failed");

        match outcome {
            Success(balance_changes) => {
                let balance_changes: Vec<_> = balance_changes
                    .iter()
                    .map(|(a, b)| (BigUint::from(*a), BigUint::from(*b)))
                    .collect();

                tb.test_tx_success(
                    swap.into(),
                    &[
                        (
                            *account_0_id,
                            AccountUpdate::UpdateBalance {
                                old_nonce: account_0.nonce,
                                new_nonce: account_0.nonce + 1,
                                balance_update: (
                                    token_0,
                                    balance_changes[0].0.clone(),
                                    balance_changes[0].1.clone(),
                                ),
                            },
                        ),
                        (
                            *recipient_1_id,
                            AccountUpdate::UpdateBalance {
                                old_nonce: recipient_1.nonce,
                                new_nonce: recipient_1.nonce,
                                balance_update: (
                                    token_0,
                                    balance_changes[1].0.clone(),
                                    balance_changes[1].1.clone(),
                                ),
                            },
                        ),
                        (
                            *account_1_id,
                            AccountUpdate::UpdateBalance {
                                old_nonce: account_1.nonce,
                                new_nonce: account_1.nonce + 1,
                                balance_update: (
                                    token_1,
                                    balance_changes[2].0.clone(),
                                    balance_changes[2].1.clone(),
                                ),
                            },
                        ),
                        (
                            *recipient_0_id,
                            AccountUpdate::UpdateBalance {
                                old_nonce: recipient_0.nonce,
                                new_nonce: recipient_0.nonce,
                                balance_update: (
                                    token_1,
                                    balance_changes[3].0.clone(),
                                    balance_changes[3].1.clone(),
                                ),
                            },
                        ),
                        (
                            *submitter_id,
                            AccountUpdate::UpdateBalance {
                                old_nonce: submitter.nonce,
                                new_nonce: submitter.nonce + 1,
                                balance_update: (
                                    fee_token,
                                    balance_changes[4].0.clone(),
                                    balance_changes[4].1.clone(),
                                ),
                            },
                        ),
                    ],
                );
            }

            Failure(message) => {
                tb.test_tx_fail(swap.into(), message);
            }
        }
    }
}

#[test]
fn swap_success() {
    let mut tb = PlasmaTestBuilder::new();

    let test_swap = TestSwap {
        accounts: (0, 1),
        recipients: (2, 3),
        submitter: 4,
        tokens: (18, 19),
        fee_token: 0,
        amounts: (50, 100),
        fee: 25,
        balances: (100, 200, 50),
        first_price: (1, 2),
        second_price: (2, 1),
        is_limit_order: (false, false),
        test_accounts: vec![
            tb.add_account(Unlocked),
            tb.add_account(Unlocked),
            tb.add_account(Unlocked),
            tb.add_account(Unlocked),
            tb.add_account(Unlocked),
        ],
    };

    test_swap.test_success(
        tb,
        Success(vec![(100, 50), (0, 50), (200, 100), (0, 100), (50, 25)]),
    );
}

#[test]
fn self_swap_fail() {
    let mut tb = PlasmaTestBuilder::new();

    let test_swap = TestSwap {
        accounts: (0, 0),
        recipients: (1, 2),
        submitter: 3,
        tokens: (18, 19),
        fee_token: 0,
        amounts: (50, 100),
        fee: 25,
        balances: (100, 200, 50),
        first_price: (1, 2),
        second_price: (2, 1),
        is_limit_order: (false, false),
        test_accounts: vec![
            tb.add_account(Unlocked),
            tb.add_account(Unlocked),
            tb.add_account(Unlocked),
            tb.add_account(Unlocked),
        ],
    };

    test_swap.test_success(tb, Failure("Self-swap is not allowed"));
}

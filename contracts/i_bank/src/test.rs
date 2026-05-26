#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

/// Helper to create test environment and contract
fn setup() -> (Env, Address, IBankContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, IBankContract);
    let client = IBankContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    (env, user, client)
}

// ============================================================================
// TEST 1: Happy Path - Deposit and Auto-Split Success
// ============================================================================
#[test]
fn test_deposit_and_split_success() {
    let (env, user, client) = setup();

    // Initialize account
    client.init_account(&user);

    // Set rules: 40% savings, 30% food, 15% transport, 15% flexible
    client.set_rules(&user, &40, &30, &15, &15, &1000);

    // Deposit 1000 units
    client.deposit(&user, &1000);

    // Verify buckets are correctly split
    let balances = client.get_balances(&user);

    assert_eq!(balances.savings, 400, "Savings should be 40% of 1000");
    assert_eq!(balances.food, 300, "Food should be 30% of 1000");
    assert_eq!(balances.transport, 150, "Transport should be 15% of 1000");
    assert_eq!(balances.flexible, 150, "Flexible should be 15% of 1000");

    // Verify total preserved
    let total = balances.savings + balances.food + balances.transport + balances.flexible;
    assert_eq!(total, 1000, "Total should equal deposit amount");
}

// ============================================================================
// TEST 2: Rule Update Blocked During Active Lock
// ============================================================================
#[test]
#[should_panic(expected = "Cannot modify rules during active self-lock")]
fn test_rule_update_blocked_during_lock() {
    let (env, user, client) = setup();

    // Initialize and set initial rules
    client.init_account(&user);
    client.set_rules(&user, &40, &30, &15, &15, &1000);

    // Activate 30-day lock (strict mode)
    let thirty_days: u64 = 30 * 24 * 60 * 60;
    client.activate_lock(&user, &thirty_days, &1);

    // Attempt to modify rules while locked - should panic
    client.set_rules(&user, &50, &25, &15, &10, &500);
}

// ============================================================================
// TEST 3: State Verification After Deposit Split
// ============================================================================
#[test]
fn test_state_after_deposit() {
    let (env, user, client) = setup();

    client.init_account(&user);
    client.set_rules(&user, &25, &25, &25, &25, &500);

    // First deposit
    client.deposit(&user, &400);

    let balances1 = client.get_balances(&user);
    assert_eq!(balances1.savings, 100);
    assert_eq!(balances1.food, 100);
    assert_eq!(balances1.transport, 100);
    assert_eq!(balances1.flexible, 100);

    // Second deposit should accumulate
    client.deposit(&user, &200);

    let balances2 = client.get_balances(&user);
    assert_eq!(balances2.savings, 150, "Savings should accumulate");
    assert_eq!(balances2.food, 150, "Food should accumulate");
    assert_eq!(balances2.transport, 150, "Transport should accumulate");
    assert_eq!(balances2.flexible, 150, "Flexible should accumulate");

    // Verify rules unchanged
    let rules = client.get_rules(&user);
    assert_eq!(rules.savings_pct, 25);
    assert_eq!(rules.daily_limit, 500);
}

// ============================================================================
// TEST 4: Spending Approval - Within Limits
// ============================================================================
#[test]
fn test_spending_approval_within_limits() {
    let (env, user, client) = setup();

    client.init_account(&user);
    client.set_rules(&user, &40, &30, &15, &15, &100); // Daily limit: 100
    client.deposit(&user, &1000);

    // Spend 50 from food bucket (within daily limit and bucket balance)
    let result = client.spend(&user, &BucketType::Food, &50);
    assert!(result, "Spending should be approved");

    // Verify bucket reduced
    let balances = client.get_balances(&user);
    assert_eq!(balances.food, 250, "Food should be reduced by 50");

    // Verify daily remaining updated
    let remaining = client.get_daily_remaining(&user);
    assert_eq!(remaining, 50, "Daily remaining should be 50");

    // Spend another 30 from flexible
    client.spend(&user, &BucketType::Flexible, &30);

    let remaining2 = client.get_daily_remaining(&user);
    assert_eq!(remaining2, 20, "Daily remaining should be 20");
}

// ============================================================================
// TEST 5: Spending Rejection - Savings Locked During Self-Lock
// ============================================================================
#[test]
#[should_panic(expected = "Savings bucket locked until self-lock expires")]
fn test_spending_rejection_savings_locked() {
    let (env, user, client) = setup();

    client.init_account(&user);
    client.set_rules(&user, &40, &30, &15, &15, &500);
    client.deposit(&user, &1000);

    // Activate 7-day lock
    let seven_days: u64 = 7 * 24 * 60 * 60;
    client.activate_lock(&user, &seven_days, &1);

    // Verify savings bucket has funds
    let balances = client.get_balances(&user);
    assert_eq!(balances.savings, 400);

    // Attempt to spend from savings - should panic
    client.spend(&user, &BucketType::Savings, &100);
}

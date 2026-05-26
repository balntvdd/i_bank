#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

// ============================================================================
// DATA TYPES
// ============================================================================

/// Spending rules defining percentage allocations and daily limit
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rules {
    pub savings_pct: u32,      // Percentage for savings (0-100)
    pub food_pct: u32,         // Percentage for food (0-100)
    pub transport_pct: u32,    // Percentage for transport (0-100)
    pub flexible_pct: u32,     // Percentage for flexible spending (0-100)
    pub daily_limit: i128,     // Maximum daily spending in token units
}

/// Self-lock configuration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockConfig {
    pub is_active: bool,       // Whether lock is currently active
    pub end_timestamp: u64,    // Unix timestamp when lock expires
    pub strictness: u32,       // 0=soft, 1=strict, 2=extreme
}

/// Individual bucket balances for a user
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Buckets {
    pub savings: i128,
    pub food: i128,
    pub transport: i128,
    pub flexible: i128,
}

/// Daily spending tracker
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DailySpending {
    pub day_timestamp: u64,    // Start of current day (truncated)
    pub amount_spent: i128,    // Total spent today
}

/// Complete user account state
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserAccount {
    pub rules: Rules,
    pub lock: LockConfig,
    pub buckets: Buckets,
    pub daily: DailySpending,
    pub initialized: bool,
}

/// Bucket identifiers for spending
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BucketType {
    Savings,
    Food,
    Transport,
    Flexible,
}

// ============================================================================
// STORAGE KEYS
// ============================================================================

const USER_ACCOUNT: Symbol = symbol_short!("USER_ACC");

// ============================================================================
// CONTRACT
// ============================================================================

#[contract]
pub struct IBankContract;

#[contractimpl]
impl IBankContract {
    // ------------------------------------------------------------------------
    // INITIALIZATION
    // ------------------------------------------------------------------------

    /// Initialize a new RuleBank account for the caller.
    /// Creates empty rules and zero balances.
    pub fn init_account(env: Env, user: Address) {
        // Require user authorization
        user.require_auth();

        // Check if account already exists
        let key = (USER_ACCOUNT, user.clone());
        if env.storage().persistent().has(&key) {
            panic!("Account already initialized");
        }

        // Create default account with zero balances and no rules
        let account = UserAccount {
            rules: Rules {
                savings_pct: 0,
                food_pct: 0,
                transport_pct: 0,
                flexible_pct: 0,
                daily_limit: 0,
            },
            lock: LockConfig {
                is_active: false,
                end_timestamp: 0,
                strictness: 0,
            },
            buckets: Buckets {
                savings: 0,
                food: 0,
                transport: 0,
                flexible: 0,
            },
            daily: DailySpending {
                day_timestamp: 0,
                amount_spent: 0,
            },
            initialized: true,
        };

        env.storage().persistent().set(&key, &account);
    }

    // ------------------------------------------------------------------------
    // RULE MANAGEMENT
    // ------------------------------------------------------------------------

    /// Set spending rules. Percentages must sum to 100.
    /// BLOCKED if self-lock is active.
    pub fn set_rules(
        env: Env,
        user: Address,
        savings_pct: u32,
        food_pct: u32,
        transport_pct: u32,
        flexible_pct: u32,
        daily_limit: i128,
    ) {
        user.require_auth();

        let key = (USER_ACCOUNT, user.clone());
        let mut account: UserAccount = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Account not initialized");

        // Check if lock is active - rules cannot be changed during lock
        if account.lock.is_active {
            let current_time = env.ledger().timestamp();
            if current_time < account.lock.end_timestamp {
                panic!("Cannot modify rules during active self-lock");
            }
            // Lock has expired, deactivate it
            account.lock.is_active = false;
        }

        // Validate percentages sum to 100
        let total = savings_pct + food_pct + transport_pct + flexible_pct;
        if total != 100 {
            panic!("Percentages must sum to 100");
        }

        // Validate daily limit is positive
        if daily_limit <= 0 {
            panic!("Daily limit must be positive");
        }

        // Update rules
        account.rules = Rules {
            savings_pct,
            food_pct,
            transport_pct,
            flexible_pct,
            daily_limit,
        };

        env.storage().persistent().set(&key, &account);
    }

    // ------------------------------------------------------------------------
    // SELF-LOCK MODE
    // ------------------------------------------------------------------------

    /// Activate self-lock mode for specified duration.
    /// Duration is in seconds. Strictness: 0=soft, 1=strict, 2=extreme
    pub fn activate_lock(env: Env, user: Address, duration_seconds: u64, strictness: u32) {
        user.require_auth();

        let key = (USER_ACCOUNT, user.clone());
        let mut account: UserAccount = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Account not initialized");

        // Cannot activate if already locked
        if account.lock.is_active {
            let current_time = env.ledger().timestamp();
            if current_time < account.lock.end_timestamp {
                panic!("Self-lock already active");
            }
        }

        // Validate strictness level
        if strictness > 2 {
            panic!("Invalid strictness level (0-2)");
        }

        // Validate duration (minimum 1 day = 86400 seconds)
        if duration_seconds < 86400 {
            panic!("Minimum lock duration is 1 day");
        }

        let current_time = env.ledger().timestamp();
        account.lock = LockConfig {
            is_active: true,
            end_timestamp: current_time + duration_seconds,
            strictness,
        };

        env.storage().persistent().set(&key, &account);
    }

    /// Check if lock has expired and deactivate if so.
    /// Returns true if lock is currently active.
    pub fn check_lock_status(env: Env, user: Address) -> bool {
        let key = (USER_ACCOUNT, user.clone());
        let mut account: UserAccount = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Account not initialized");

        if account.lock.is_active {
            let current_time = env.ledger().timestamp();
            if current_time >= account.lock.end_timestamp {
                account.lock.is_active = false;
                env.storage().persistent().set(&key, &account);
                return false;
            }
            return true;
        }
        false
    }

    // ------------------------------------------------------------------------
    // DEPOSIT & AUTO-SPLIT
    // ------------------------------------------------------------------------

    /// Deposit funds and automatically split according to rules.
    /// In production, this would integrate with Stellar asset transfers.
    /// For MVP, we track balances internally.
    pub fn deposit(env: Env, user: Address, amount: i128) {
        user.require_auth();

        if amount <= 0 {
            panic!("Deposit amount must be positive");
        }

        let key = (USER_ACCOUNT, user.clone());
        let mut account: UserAccount = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Account not initialized");

        // Ensure rules are set (total must equal 100%)
        let total_pct = account.rules.savings_pct
            + account.rules.food_pct
            + account.rules.transport_pct
            + account.rules.flexible_pct;

        if total_pct != 100 {
            panic!("Rules must be set before depositing");
        }

        // Calculate splits using integer math to avoid precision loss
        // We allocate based on percentages, handling remainder in flexible
        let savings_amount = (amount * account.rules.savings_pct as i128) / 100;
        let food_amount = (amount * account.rules.food_pct as i128) / 100;
        let transport_amount = (amount * account.rules.transport_pct as i128) / 100;
        // Flexible gets the remainder to ensure no dust is lost
        let flexible_amount = amount - savings_amount - food_amount - transport_amount;

        // Add to buckets
        account.buckets.savings += savings_amount;
        account.buckets.food += food_amount;
        account.buckets.transport += transport_amount;
        account.buckets.flexible += flexible_amount;

        env.storage().persistent().set(&key, &account);
    }

    // ------------------------------------------------------------------------
    // SPENDING REQUESTS
    // ------------------------------------------------------------------------

    /// Request to spend from a specific bucket.
    /// Validates against daily limit and bucket balance.
    /// Savings bucket is BLOCKED during active lock (strict/extreme modes).
    pub fn spend(env: Env, user: Address, bucket: BucketType, amount: i128) -> bool {
        user.require_auth();

        if amount <= 0 {
            panic!("Spend amount must be positive");
        }

        let key = (USER_ACCOUNT, user.clone());
        let mut account: UserAccount = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Account not initialized");

        let current_time = env.ledger().timestamp();

        // Check lock status for savings bucket
        if account.lock.is_active && current_time < account.lock.end_timestamp {
            match bucket {
                BucketType::Savings => {
                    // Savings is always locked during self-lock
                    panic!("Savings bucket locked until self-lock expires");
                }
                _ => {
                    // Other buckets allowed based on strictness
                    // Extreme mode: no spending at all from any bucket
                    if account.lock.strictness == 2 {
                        panic!("Extreme lock: all spending frozen");
                    }
                }
            }
        } else if account.lock.is_active {
            // Lock expired, deactivate
            account.lock.is_active = false;
        }

        // Reset daily spending if new day
        let day_start = current_time - (current_time % 86400); // Truncate to day
        if account.daily.day_timestamp != day_start {
            account.daily.day_timestamp = day_start;
            account.daily.amount_spent = 0;
        }

        // Check daily spending limit
        if account.daily.amount_spent + amount > account.rules.daily_limit {
            panic!("Daily spending limit exceeded");
        }

        // Check bucket balance
        let bucket_balance = match bucket {
            BucketType::Savings => account.buckets.savings,
            BucketType::Food => account.buckets.food,
            BucketType::Transport => account.buckets.transport,
            BucketType::Flexible => account.buckets.flexible,
        };

        if bucket_balance < amount {
            panic!("Insufficient bucket balance");
        }

        // Deduct from bucket
        match bucket {
            BucketType::Savings => account.buckets.savings -= amount,
            BucketType::Food => account.buckets.food -= amount,
            BucketType::Transport => account.buckets.transport -= amount,
            BucketType::Flexible => account.buckets.flexible -= amount,
        }

        // Update daily spending
        account.daily.amount_spent += amount;

        env.storage().persistent().set(&key, &account);

        true
    }

    // ------------------------------------------------------------------------
    // VIEW FUNCTIONS
    // ------------------------------------------------------------------------

    /// Get current account state
    pub fn get_account(env: Env, user: Address) -> UserAccount {
        let key = (USER_ACCOUNT, user);
        env.storage()
            .persistent()
            .get(&key)
            .expect("Account not initialized")
    }

    /// Get bucket balances only
    pub fn get_balances(env: Env, user: Address) -> Buckets {
        let key = (USER_ACCOUNT, user);
        let account: UserAccount = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Account not initialized");
        account.buckets
    }

    /// Get current rules
    pub fn get_rules(env: Env, user: Address) -> Rules {
        let key = (USER_ACCOUNT, user);
        let account: UserAccount = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Account not initialized");
        account.rules
    }

    /// Get remaining daily spending allowance
    pub fn get_daily_remaining(env: Env, user: Address) -> i128 {
        let key = (USER_ACCOUNT, user);
        let account: UserAccount = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Account not initialized");

        let current_time = env.ledger().timestamp();
        let day_start = current_time - (current_time % 86400);

        // If new day, full limit available
        if account.daily.day_timestamp != day_start {
            return account.rules.daily_limit;
        }

        // Otherwise, calculate remaining
        let remaining = account.rules.daily_limit - account.daily.amount_spent;
        if remaining < 0 {
            0
        } else {
            remaining
        }
    }

    /// Get lock expiry timestamp (0 if not locked)
    pub fn get_lock_expiry(env: Env, user: Address) -> u64 {
        let key = (USER_ACCOUNT, user);
        let account: UserAccount = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Account not initialized");

        if account.lock.is_active {
            account.lock.end_timestamp
        } else {
            0
        }
    }
}

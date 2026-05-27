#![no_std]

mod errors;
mod fraud_detection;
mod helpers;
mod liquidity_mining;
mod oracle;
mod staking_derivatives;
mod types;
mod vouch;
mod vouch_snapshot;

use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, Env, String, Vec};

#[cfg(test)]
mod withdrawal_queue_test;

use crate::errors::ContractError;
use crate::helpers::{config, get_active_loan_record, has_active_loan, require_allowed_token, require_not_paused, require_admin_approval};
use crate::types::{
    Config, DataKey, LoanRecord, LoanStatus, QueuedWithdrawal, SlashRecord, VouchRecord,
    DEFAULT_LIQUIDITY_MINING_RATE_BPS, DEFAULT_LOAN_DURATION, DEFAULT_MAX_LOAN_TO_STAKE_RATIO,
    DEFAULT_MAX_VOUCHERS, DEFAULT_MIN_LOAN_AMOUNT, DEFAULT_MIN_VOUCH_AGE_SECS, DEFAULT_SLASH_BPS,
    DEFAULT_YIELD_BPS,
};

#[contract]
pub struct QuorumCreditContract;

#[contractimpl]
impl QuorumCreditContract {
    // ─────────────────────────────────────────────
    // Initialization
    // ─────────────────────────────────────────────

    pub fn initialize(
        env: Env,
        deployer: Address,
        admins: Vec<Address>,
        admin_threshold: u32,
        token: Address,
    ) -> Result<(), ContractError> {
        deployer.require_auth();

        if env.storage().instance().has(&DataKey::Config) {
            return Err(ContractError::AlreadyInitialized);
        }

        if admins.is_empty() || admin_threshold == 0 || admin_threshold > admins.len() {
            return Err(ContractError::InvalidAmount);
        }

        env.storage().instance().set(&DataKey::Deployer, &deployer);
        env.storage().instance().set(
            &DataKey::Config,
            &Config {
                admins,
                admin_threshold,
                token,
                allowed_tokens: Vec::new(&env),
                yield_bps: DEFAULT_YIELD_BPS,
                slash_bps: DEFAULT_SLASH_BPS,
                max_vouchers: DEFAULT_MAX_VOUCHERS,
                min_loan_amount: DEFAULT_MIN_LOAN_AMOUNT,
                loan_duration: DEFAULT_LOAN_DURATION,
                max_loan_to_stake_ratio: DEFAULT_MAX_LOAN_TO_STAKE_RATIO,
                grace_period: 0,
                min_vouch_age_secs: DEFAULT_MIN_VOUCH_AGE_SECS,
                prepayment_penalty_bps: 0,
                liquidity_mining_rate_bps: DEFAULT_LIQUIDITY_MINING_RATE_BPS,
                partial_default_threshold_bps: 0,
            },
        );

        Ok(())
    }

    // ─────────────────────────────────────────────
    // Core Vouching
    // ─────────────────────────────────────────────

    pub fn vouch(
        env: Env,
        voucher: Address,
        borrower: Address,
        stake: i128,
        token: Address,
    ) -> Result<(), ContractError> {
        vouch::vouch(env, voucher, borrower, stake, token)
    }

    pub fn batch_vouch(
        env: Env,
        voucher: Address,
        borrowers: Vec<Address>,
        stakes: Vec<i128>,
        token: Address,
    ) -> Result<(), ContractError> {
        vouch::batch_vouch(env, voucher, borrowers, stakes, token)
    }

    // ─────────────────────────────────────────────
    // Stake Management
    // ─────────────────────────────────────────────

    pub fn increase_stake(
        env: Env,
        voucher: Address,
        borrower: Address,
        additional: i128,
    ) -> Result<(), ContractError> {
        vouch::increase_stake(env, voucher, borrower, additional)
    }

    /// Decrease stake. If borrower has an active loan, queues the withdrawal.
    pub fn decrease_stake(
        env: Env,
        voucher: Address,
        borrower: Address,
        amount: i128,
    ) -> Result<(), ContractError> {
        vouch::decrease_stake(env, voucher, borrower, amount)
    }

    /// Fully withdraw a vouch. If borrower has an active loan, queues the withdrawal.
    pub fn withdraw_vouch(
        env: Env,
        voucher: Address,
        borrower: Address,
    ) -> Result<(), ContractError> {
        vouch::withdraw_vouch(env, voucher, borrower)
    }

    // ─────────────────────────────────────────────
    // Withdrawal Queue
    // ─────────────────────────────────────────────

    /// Queue a withdrawal during an active loan.
    pub fn request_withdrawal(
        env: Env,
        voucher: Address,
        borrower: Address,
        priority_fee: i128,
    ) -> Result<(), ContractError> {
        vouch::request_withdrawal(env, voucher, borrower, priority_fee)
    }

    /// Partial withdrawal: withdraw up to 50% of stake during an active loan.
    pub fn partial_withdraw(
        env: Env,
        voucher: Address,
        borrower: Address,
    ) -> Result<(), ContractError> {
        vouch::partial_withdraw(env, voucher, borrower)
    }

    /// Get the pending withdrawal queue for a borrower.
    pub fn get_withdrawal_queue(env: Env, borrower: Address) -> Vec<QueuedWithdrawal> {
        vouch::get_withdrawal_queue(env, borrower)
    }

    // ─────────────────────────────────────────────
    // Loans
    // ─────────────────────────────────────────────

    pub fn request_loan(
        env: Env,
        borrower: Address,
        amount: i128,
        threshold: i128,
        loan_purpose: String,
        token_addr: Address,
    ) -> Result<(), ContractError> {
        borrower.require_auth();
        require_not_paused(&env)?;

        if has_active_loan(&env, &borrower) {
            return Err(ContractError::ActiveLoanExists);
        }

        let token_client = require_allowed_token(&env, &token_addr)?;
        let cfg = config(&env);

        if amount < cfg.min_loan_amount {
            return Err(ContractError::LoanBelowMinAmount);
        }

        let vouches: Vec<VouchRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::Vouches(borrower.clone()))
            .unwrap_or(Vec::new(&env));

        let total_stake: i128 = vouches
            .iter()
            .filter(|v| v.token == token_addr)
            .map(|v| v.stake)
            .sum();

        if total_stake < threshold {
            return Err(ContractError::InsufficientFunds);
        }

        let now = env.ledger().timestamp();
        let loan_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::LoanCounter)
            .unwrap_or(0u64)
            + 1;
        env.storage()
            .persistent()
            .set(&DataKey::LoanCounter, &loan_id);

        let total_yield = amount * cfg.yield_bps / 10_000;

        let loan = LoanRecord {
            id: loan_id,
            borrower: borrower.clone(),
            co_borrowers: Vec::new(&env),
            amount,
            amount_repaid: 0,
            total_yield,
            status: LoanStatus::Active,
            created_at: now,
            disbursement_timestamp: now,
            repayment_timestamp: None,
            deadline: now + cfg.loan_duration,
            loan_purpose,
            token_address: token_addr.clone(),
            amortization_schedule: Vec::new(&env),
            reminder_sent: false,
            risk_score: 0,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Loan(loan_id), &loan);
        env.storage()
            .persistent()
            .set(&DataKey::ActiveLoan(borrower.clone()), &loan_id);

        token_client.transfer(&env.current_contract_address(), &borrower, &amount);

        env.events().publish(
            (symbol_short!("loan"), symbol_short!("created")),
            (borrower, amount),
        );

        Ok(())
    }

    pub fn repay(env: Env, borrower: Address, payment: i128) -> Result<(), ContractError> {
        borrower.require_auth();
        require_not_paused(&env)?;

        let mut loan = get_active_loan_record(&env, &borrower)?;

        if payment <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let total_owed = loan.amount + loan.total_yield;
        let outstanding = total_owed - loan.amount_repaid;

        if payment > outstanding {
            return Err(ContractError::InvalidAmount);
        }

        let token_client = require_allowed_token(&env, &loan.token_address)?;
        token_client.transfer(&borrower, &env.current_contract_address(), &payment);

        loan.amount_repaid += payment;

        if loan.amount_repaid >= total_owed {
            loan.status = LoanStatus::Repaid;
            loan.repayment_timestamp = Some(env.ledger().timestamp());

            let vouches: Vec<VouchRecord> = env
                .storage()
                .persistent()
                .get(&DataKey::Vouches(borrower.clone()))
                .unwrap_or(Vec::new(&env));

            let total_stake: i128 = vouches
                .iter()
                .filter(|v| v.token == loan.token_address)
                .map(|v| v.stake)
                .sum();

            for v in vouches.iter() {
                if v.token != loan.token_address {
                    continue;
                }
                let yield_share = if total_stake > 0 {
                    loan.total_yield * v.stake / total_stake
                } else {
                    0
                };
                token_client.transfer(
                    &env.current_contract_address(),
                    &v.voucher,
                    &(v.stake + yield_share),
                );
            }

            vouch::process_withdrawal_queue(&env, &borrower);

            env.storage()
                .persistent()
                .remove(&DataKey::ActiveLoan(borrower.clone()));
            env.storage()
                .persistent()
                .remove(&DataKey::Vouches(borrower.clone()));

            env.events().publish(
                (symbol_short!("loan"), symbol_short!("repaid")),
                (borrower.clone(), loan.amount),
            );
        }

        env.storage()
            .persistent()
            .set(&DataKey::Loan(loan.id), &loan);

        Ok(())
    }

    // ─────────────────────────────────────────────
    // #665: Batch Repayment
    // ─────────────────────────────────────────────

    /// Batch multiple repayments into a single transaction.
    ///
    /// Each entry in `borrowers` / `payments` is processed in order using the same
    /// logic as `repay()`. The batch is NOT atomic — each repayment is applied
    /// independently. If one fails the error is returned immediately and subsequent
    /// entries are not processed.
    ///
    /// # Arguments
    /// * `borrowers` - Ordered list of borrower addresses to repay on behalf of
    /// * `payments`  - Corresponding payment amounts in stroops (must be same length)
    pub fn batch_repay(
        env: Env,
        borrowers: Vec<Address>,
        payments: Vec<i128>,
    ) -> Result<(), ContractError> {
        require_not_paused(&env)?;

        if borrowers.len() != payments.len() || borrowers.is_empty() {
            return Err(ContractError::InvalidAmount);
        }

        for i in 0..borrowers.len() {
            let borrower = borrowers.get(i).unwrap();
            let payment = payments.get(i).unwrap();

            borrower.require_auth();

            let mut loan = get_active_loan_record(&env, &borrower)?;

            if payment <= 0 {
                return Err(ContractError::InvalidAmount);
            }

            let total_owed = loan.amount + loan.total_yield;
            let outstanding = total_owed - loan.amount_repaid;

            if payment > outstanding {
                return Err(ContractError::InvalidAmount);
            }

            let token_client = require_allowed_token(&env, &loan.token_address)?;
            token_client.transfer(&borrower, &env.current_contract_address(), &payment);

            loan.amount_repaid += payment;

            if loan.amount_repaid >= total_owed {
                loan.status = LoanStatus::Repaid;
                loan.repayment_timestamp = Some(env.ledger().timestamp());

                let vouches: Vec<VouchRecord> = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Vouches(borrower.clone()))
                    .unwrap_or(Vec::new(&env));

                let total_stake: i128 = vouches
                    .iter()
                    .filter(|v| v.token == loan.token_address)
                    .map(|v| v.stake)
                    .sum();

                for v in vouches.iter() {
                    if v.token != loan.token_address {
                        continue;
                    }
                    let yield_share = if total_stake > 0 {
                        loan.total_yield * v.stake / total_stake
                    } else {
                        0
                    };
                    token_client.transfer(
                        &env.current_contract_address(),
                        &v.voucher,
                        &(v.stake + yield_share),
                    );
                }

                vouch::process_withdrawal_queue(&env, &borrower);

                env.storage()
                    .persistent()
                    .remove(&DataKey::ActiveLoan(borrower.clone()));
                env.storage()
                    .persistent()
                    .remove(&DataKey::Vouches(borrower.clone()));

                env.events().publish(
                    (symbol_short!("loan"), symbol_short!("repaid")),
                    (borrower.clone(), loan.amount),
                );
            }

            env.storage()
                .persistent()
                .set(&DataKey::Loan(loan.id), &loan);

            env.events().publish(
                (symbol_short!("loan"), symbol_short!("batch_pay")),
                (borrower, payment),
            );
        }

        Ok(())
    }

    // ─────────────────────────────────────────────
    // #663: Partial Default Handling
    // ─────────────────────────────────────────────

    /// Mark a loan as a partial default when the borrower has repaid some but not
    /// enough to meet the `partial_default_threshold_bps` in Config.
    ///
    /// Called by admin after the loan deadline has passed. If `partial_default_threshold_bps`
    /// is 0 (disabled), this returns `InvalidStateTransition`.
    ///
    /// Vouchers are slashed proportionally to the unpaid fraction of the loan.
    pub fn mark_partial_default(
        env: Env,
        admin_signers: Vec<Address>,
        borrower: Address,
    ) -> Result<(), ContractError> {
        require_not_paused(&env)?;
        require_admin_approval(&env, &admin_signers)?;

        let cfg = config(&env);

        if cfg.partial_default_threshold_bps == 0 {
            return Err(ContractError::InvalidStateTransition);
        }

        let mut loan = get_active_loan_record(&env, &borrower)?;

        if loan.status != LoanStatus::Active {
            return Err(ContractError::InvalidStateTransition);
        }

        let total_owed = loan.amount + loan.total_yield;
        // repaid_bps = amount_repaid * 10_000 / total_owed
        let repaid_bps = if total_owed > 0 {
            loan.amount_repaid * 10_000 / total_owed
        } else {
            0
        };

        // If borrower repaid >= threshold, this is not a partial default
        if repaid_bps >= cfg.partial_default_threshold_bps as i128 {
            return Err(ContractError::InvalidStateTransition);
        }

        loan.status = LoanStatus::PartialDefault;

        // Slash vouchers proportionally to the unpaid fraction
        let unpaid_bps = 10_000 - repaid_bps;
        let effective_slash_bps = cfg.slash_bps * unpaid_bps / 10_000;

        let vouches: Vec<VouchRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::Vouches(borrower.clone()))
            .unwrap_or(Vec::new(&env));

        let token_client = require_allowed_token(&env, &loan.token_address)?;
        let mut total_slashed: i128 = 0;

        for v in vouches.iter() {
            if v.token != loan.token_address {
                continue;
            }
            let slash_amount = v.stake * effective_slash_bps / 10_000;
            total_slashed += slash_amount;
            // Return remaining stake to voucher
            let returned = v.stake - slash_amount;
            if returned > 0 {
                token_client.transfer(&env.current_contract_address(), &v.voucher, &returned);
            }
        }

        // Record slash with forgiveness fields empty
        let slash_record = SlashRecord {
            loan_id: loan.id,
            borrower: borrower.clone(),
            total_slashed,
            slash_timestamp: env.ledger().timestamp(),
            forgiven: false,
            forgiveness_reason: String::from_str(&env, ""),
            forgiven_at: 0,
        };
        env.storage()
            .persistent()
            .set(&DataKey::SlashRecord(loan.id), &slash_record);

        // Increment partial default count
        let prev: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::PartialDefaultCount(borrower.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::PartialDefaultCount(borrower.clone()), &(prev + 1));

        env.storage()
            .persistent()
            .remove(&DataKey::ActiveLoan(borrower.clone()));
        env.storage()
            .persistent()
            .remove(&DataKey::Vouches(borrower.clone()));

        env.storage()
            .persistent()
            .set(&DataKey::Loan(loan.id), &loan);

        env.events().publish(
            (symbol_short!("loan"), symbol_short!("part_def")),
            (borrower, total_slashed),
        );

        Ok(())
    }

    // ─────────────────────────────────────────────
    // #664: Default Forgiveness Program
    // ─────────────────────────────────────────────

    /// Admin forgives a default (Defaulted or PartialDefault) for hardship cases.
    ///
    /// Marks the loan as `ForgivenDefault` and records the `forgiveness_reason`
    /// in the `SlashRecord`. Does NOT reverse the slash — funds already distributed
    /// to the slash treasury remain there. The borrower's default count is decremented
    /// so future loan eligibility is restored.
    ///
    /// # Arguments
    /// * `admin_signers`      - Admin addresses meeting the threshold
    /// * `borrower`           - Borrower whose default is being forgiven
    /// * `loan_id`            - ID of the defaulted loan
    /// * `forgiveness_reason` - Human-readable reason for forgiveness
    pub fn forgive_default(
        env: Env,
        admin_signers: Vec<Address>,
        borrower: Address,
        loan_id: u64,
        forgiveness_reason: String,
    ) -> Result<(), ContractError> {
        require_not_paused(&env)?;
        require_admin_approval(&env, &admin_signers)?;

        let mut loan: LoanRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Loan(loan_id))
            .ok_or(ContractError::NoActiveLoan)?;

        if loan.borrower != borrower {
            return Err(ContractError::UnauthorizedCaller);
        }

        if loan.status != LoanStatus::Defaulted && loan.status != LoanStatus::PartialDefault {
            return Err(ContractError::InvalidStateTransition);
        }

        loan.status = LoanStatus::ForgivenDefault;
        env.storage()
            .persistent()
            .set(&DataKey::Loan(loan_id), &loan);

        // Update slash record with forgiveness info
        let mut slash_record: SlashRecord = env
            .storage()
            .persistent()
            .get(&DataKey::SlashRecord(loan_id))
            .unwrap_or(SlashRecord {
                loan_id,
                borrower: borrower.clone(),
                total_slashed: 0,
                slash_timestamp: 0,
                forgiven: false,
                forgiveness_reason: String::from_str(&env, ""),
                forgiven_at: 0,
            });

        slash_record.forgiven = true;
        slash_record.forgiveness_reason = forgiveness_reason.clone();
        slash_record.forgiven_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::SlashRecord(loan_id), &slash_record);

        // Decrement default count to restore borrower eligibility
        let default_count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::DefaultCount(borrower.clone()))
            .unwrap_or(0);
        if default_count > 0 {
            env.storage()
                .persistent()
                .set(&DataKey::DefaultCount(borrower.clone()), &(default_count - 1));
        }

        env.events().publish(
            (symbol_short!("loan"), symbol_short!("forgiven")),
            (borrower, loan_id, forgiveness_reason),
        );

        Ok(())
    }

    /// Get the slash record for a loan (includes forgiveness info if applicable).
    pub fn get_slash_record(env: Env, loan_id: u64) -> Option<SlashRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::SlashRecord(loan_id))
    }

    /// Get the partial default count for a borrower.
    pub fn get_partial_default_count(env: Env, borrower: Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::PartialDefaultCount(borrower))
            .unwrap_or(0)
    }

    // ─────────────────────────────────────────────
    // Queries
    // ─────────────────────────────────────────────

    pub fn get_loan(env: Env, borrower: Address) -> Option<LoanRecord> {
        let loan_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ActiveLoan(borrower.clone()))?;
        env.storage().persistent().get(&DataKey::Loan(loan_id))
    }

    pub fn get_vouches(env: Env, borrower: Address) -> Vec<VouchRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Vouches(borrower))
            .unwrap_or(Vec::new(&env))
    }

    pub fn vouch_exists(env: Env, voucher: Address, borrower: Address) -> bool {
        let vouches: Vec<VouchRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::Vouches(borrower))
            .unwrap_or(Vec::new(&env));
        vouches.iter().any(|v| v.voucher == voucher)
    }
}

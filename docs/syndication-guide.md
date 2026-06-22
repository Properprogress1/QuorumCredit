# Loan Pool Syndication for Multi-Borrower Loans

## Overview

The Loan Pool Syndication system enables multiple borrowers to pool together and share responsibility for a single loan. This allows groups of borrowers to access larger loan amounts, share collateral requirements, and distribute repayment obligations among syndicate members.

## Syndication Roles

### Lead Borrower
- Primary contact and decision maker for the syndication
- Can cancel the syndication before loan disbursement
- Receives the loan disbursement
- Primary responsible party for loan repayment

### Co-Borrower
- Shares loan responsibility with the lead borrower
- Contributes collateral and vouches
- Required to repay if lead borrower defaults
- Credit score affected by syndication performance

### Guarantor
- Provides additional collateral but is not a borrower
- Does not receive loan funds
- Collateral is slashed if syndication defaults
- Credit score not affected by syndication performance

## Syndication Lifecycle

### 1. Formation (Forming Status)
- Lead borrower creates syndication
- Members join with specified roles and shares
- Members approve the syndication
- Syndication becomes ready when minimum approvals reached

### 2. Ready (Ready Status)
- All required approvals received
- Syndication can request loan disbursement
- Members can still leave (if minimum members maintained)

### 3. Active (Active Status)
- Loan disbursed to lead borrower
- Members cannot leave
- Any member can contribute to repayment
- Syndication is subject to default if not repaid

### 4. Repaid (Repaid Status)
- Loan fully repaid
- All members' credit scores updated
- Collateral returned to members
- Syndication closed

### 5. Cancelled (Cancelled Status)
- Syndication cancelled before loan disbursement
- Can be cancelled by lead borrower
- Automatically cancelled if members drop below minimum
- No loan funds disbursed

### 6. Defaulted (Defaulted Status)
- Loan not repaid by deadline
- All collateral slashed from members
- Default counts updated for borrowers
- Syndication closed

## Data Structures

### SyndicationMember

```rust
pub struct SyndicationMember {
    pub address: Address,           // Member address
    pub role: SyndicationRole,      // Role in syndication
    pub share_bps: u32,             // Share of loan (basis points)
    pub collateral: i128,          // Collateral contributed (stroops)
    pub vouch_stake: i128,         // Vouches contributed (stroops)
    pub approved: bool,            // Whether member has approved
    pub joined_at: u64,            // Timestamp when member joined
}
```

### LoanSyndication

```rust
pub struct LoanSyndication {
    pub syndication_id: u64,                    // Unique syndication ID
    pub loan_id: Option<u64>,                   // Associated loan ID
    pub members: Vec<SyndicationMember>,        // Syndication members
    pub total_amount: i128,                     // Total loan amount (stroops)
    pub total_collateral: i128,                 // Total collateral (stroops)
    pub total_vouch_stake: i128,                // Total vouch stake (stroops)
    pub loan_purpose: soroban_sdk::String,      // Loan purpose
    pub token_address: Address,                 // Token address
    pub created_at: u64,                        // Creation timestamp
    pub disbursed_at: Option<u64>,              // Disbursement timestamp
    pub status: SyndicationStatus,             // Current status
    pub min_approvals: u32,                     // Minimum approvals required
    pub approval_count: u32,                    // Current approval count
}
```

### SyndicationStatus

```rust
pub enum SyndicationStatus {
    Forming,      // Syndication is being formed
    Ready,        // Syndication is ready for loan disbursement
    Active,       // Loan has been disbursed
    Repaid,       // Loan has been fully repaid
    Cancelled,    // Syndication has been cancelled
    Defaulted,    // Syndication has defaulted
}
```

### SyndicationConfig

```rust
pub struct SyndicationConfig {
    pub max_members: u32,                // Maximum members in syndication
    pub min_members: u32,                // Minimum members required
    pub min_approval_percentage: u32,    // Minimum approvals (basis points)
    pub max_loan_amount: i128,           // Maximum loan amount (stroops)
    pub syndication_fee_bps: u32,        // Syndication fee (basis points)
}
```

## Contract Functions

### Syndication Management

#### `create_syndication`

Create a new loan syndication.

```rust
pub fn create_syndication(
    env: Env,
    creator: Address,
    loan_purpose: soroban_sdk::String,
    token_address: Address,
    total_amount: i128,
) -> Result<u64, ContractError>
```

**Parameters:**
- `creator`: Address creating the syndication
- `loan_purpose`: Description of loan purpose
- `token_address`: Token contract address
- `total_amount`: Total loan amount requested (in stroops)

**Returns:** Syndication ID

**Behavior:** Creates a new syndication in Forming status.

#### `join_syndication`

Join a syndication as a member.

```rust
pub fn join_syndication(
    env: Env,
    syndication_id: u64,
    member: Address,
    role: SyndicationRole,
    share_bps: u32,
    collateral: i128,
    vouch_stake: i128,
) -> Result<(), ContractError>
```

**Parameters:**
- `syndication_id`: Syndication to join
- `member`: Address joining the syndication
- `role`: Role in the syndication
- `share_bps`: Share of loan (basis points, 0-10000)
- `collateral`: Collateral contributed (in stroops)
- `vouch_stake`: Vouches contributed (in stroops)

**Behavior:** Adds member to syndication with specified role and contributions.

#### `approve_syndication`

Approve a syndication (member approval).

```rust
pub fn approve_syndication(
    env: Env,
    syndication_id: u64,
    member: Address,
) -> Result<(), ContractError>
```

**Parameters:**
- `syndication_id`: Syndication to approve
- `member`: Address approving the syndication

**Behavior:** Marks member as approved. Syndication becomes Ready when minimum approvals reached.

#### `leave_syndication`

Leave a syndication (only allowed before loan disbursement).

```rust
pub fn leave_syndication(
    env: Env,
    syndication_id: u64,
    member: Address,
) -> Result<(), ContractError>
```

**Parameters:**
- `syndication_id`: Syndication to leave
- `member`: Address leaving the syndication

**Behavior:** Removes member from syndication. Syndication cancelled if members drop below minimum.

#### `cancel_syndication`

Cancel a syndication (only by lead borrower).

```rust
pub fn cancel_syndication(
    env: Env,
    syndication_id: u64,
    caller: Address,
) -> Result<(), ContractError>
```

**Parameters:**
- `syndication_id`: Syndication to cancel
- `caller`: Address cancelling (must be lead borrower)

**Behavior:** Cancels syndication. Only allowed before loan disbursement.

### Loan Operations

#### `request_syndication_loan`

Request a loan for a syndication (disburse the loan).

```rust
pub fn request_syndication_loan(
    env: Env,
    syndication_id: u64,
    lead_borrower: Address,
) -> Result<u64, ContractError>
```

**Parameters:**
- `syndication_id`: Syndication requesting loan
- `lead_borrower`: Lead borrower address

**Returns:** Loan ID

**Behavior:** Disburses loan to lead borrower. Syndication must be in Ready status.

#### `repay_syndication_loan`

Repay a syndication loan (any member can contribute).

```rust
pub fn repay_syndication_loan(
    env: Env,
    syndication_id: u64,
    repayer: Address,
    amount: i128,
) -> Result<(), ContractError>
```

**Parameters:**
- `syndication_id`: Syndication to repay
- `repayer`: Address making repayment
- `amount`: Amount to repay (in stroops)

**Behavior:** Any member can contribute to repayment. Loan marked as repaid when fully paid.

#### `handle_syndication_default`

Handle syndication default (slash collateral from all members).

```rust
pub fn handle_syndication_default(
    env: Env,
    syndication_id: u64,
    caller: Address,
) -> Result<(), ContractError>
```

**Parameters:**
- `syndication_id`: Syndication in default
- `caller`: Address triggering default handling

**Behavior:** Slashes collateral from all members. Updates default counts for borrowers.

### View Functions

#### `get_syndication`

Get syndication by ID.

```rust
pub fn get_syndication(env: Env, syndication_id: u64) -> Option<LoanSyndication>
```

**Returns:** Syndication record, or None if not found.

#### `get_syndication_member`

Get syndication member.

```rust
pub fn get_syndication_member(
    env: Env,
    syndication_id: u64,
    member: Address,
) -> Option<SyndicationMember>
```

**Returns:** Syndication member, or None if not found.

#### `get_syndication_config_view`

Get syndication configuration.

```rust
pub fn get_syndication_config_view(env: Env) -> SyndicationConfig
```

**Returns:** Current syndication configuration.

#### `get_syndication_count`

Get syndication count.

```rust
pub fn get_syndication_count(env: Env) -> u64
```

**Returns:** Total number of syndications created.

### Configuration

#### `set_syndication_config`

Set syndication configuration (admin only).

```rust
pub fn set_syndication_config(
    env: Env,
    admin_signers: Vec<Address>,
    config: SyndicationConfig,
) -> Result<(), ContractError>
```

**Parameters:**
- `admin_signers`: Admin addresses approving the change
- `config`: New syndication configuration

**Requirements:** Admin approval (multi-sig)

**Validation:** Configuration must be valid (min_members >= 2, max_members >= min_members, min_approval_percentage between 5000-10000).

## Workflow Example

### 1. Create Syndication

```rust
let syndication_id = client.create_syndication(
    &creator,
    &soroban_sdk::String::from_str(&env, "Business expansion"),
    &token_address,
    &100_000_000, // 10 XLM
).unwrap();
```

### 2. Add Members

```rust
// Add lead borrower
client.join_syndication(
    &syndication_id,
    &lead_borrower,
    SyndicationRole::LeadBorrower,
    &5000, // 50% share
    &10_000_000, // 1 XLM collateral
    &5_000_000, // 0.5 XLM vouch
).unwrap();

// Add co-borrower
client.join_syndication(
    &syndication_id,
    &co_borrower,
    SyndicationRole::CoBorrower,
    &5000, // 50% share
    &10_000_000, // 1 XLM collateral
    &5_000_000, // 0.5 XLM vouch
).unwrap();
```

### 3. Approve Syndication

```rust
client.approve_syndication(&syndication_id, &lead_borrower).unwrap();
client.approve_syndication(&syndication_id, &co_borrower).unwrap();
```

### 4. Request Loan

```rust
let loan_id = client.request_syndication_loan(
    &syndication_id,
    &lead_borrower,
).unwrap();
```

### 5. Repay Loan

```rust
// Any member can contribute
client.repay_syndication_loan(
    &syndication_id,
    &lead_borrower,
    &50_000_000, // 5 XLM
).unwrap();

client.repay_syndication_loan(
    &syndication_id,
    &co_borrower,
    &50_000_000, // 5 XLM
).unwrap();
```

## Default Configuration

```rust
pub const DEFAULT_SYNDICATION_CONFIG: SyndicationConfig = SyndicationConfig {
    max_members: 10,
    min_members: 2,
    min_approval_percentage: 7500, // 75%
    max_loan_amount: 1_000_000_000_000, // 10 million XLM
    syndication_fee_bps: 100, // 1%
};
```

## Testing

Comprehensive tests are available in `src/syndication_test.rs`:

- `test_create_syndication` - Test syndication creation
- `test_join_syndication` - Test member joining
- `test_join_syndication_invalid_share` - Test invalid share validation
- `test_approve_syndication` - Test syndication approval
- `test_leave_syndication` - Test member leaving
- `test_cancel_syndication` - Test syndication cancellation
- `test_cancel_syndication_unauthorized` - Test unauthorized cancellation
- `test_set_syndication_config` - Test configuration update
- `test_set_syndication_config_invalid` - Test invalid configuration validation
- `test_get_syndication_count` - Test syndication count
- `test_syndication_status_transitions` - Test status transitions
- `test_default_syndication_config` - Test default configuration

Run tests with:
```bash
cargo test syndication_test
```

## Event Topics

The following events are published by the syndication system:

- `("syndication", "created")` - Syndication created (syndication_id, creator, total_amount)
- `("syndication", "joined")` - Member joined (syndication_id, member, role)
- `("syndication", "approved")` - Syndication approved (syndication_id, member, approval_count)
- `("syndication", "left")` - Member left (syndication_id, member)
- `("syndication", "cancelled")` - Syndication cancelled (syndication_id, caller)
- `("syndication", "loan_disbursed")` - Loan disbursed (syndication_id, loan_id, amount)
- `("syndication", "repayment")` - Repayment made (syndication_id, repayer, amount)
- `("syndication", "defaulted")` - Syndication defaulted (syndication_id, loan_id, collateral_slashed)
- `("syndication", "config")` - Configuration updated (admin_address)

## Error Codes

- `SyndicationNotFound` - Syndication not found
- `SyndicationMemberNotFound` - Syndication member not found
- `SyndicationHasLoan` - Syndication already has a loan
- `InvalidSyndicationStatus` - Syndication is not in the correct status
- `SyndicationMemberExists` - Syndication member already exists
- `InsufficientSyndicationApprovals` - Syndication has insufficient approvals
- `SyndicationMaxMembersExceeded` - Syndication has too many members
- `SyndicationMinMembersNotMet` - Syndication has too few members
- `InvalidSyndicationShare` - Invalid syndication share percentage
- `InvalidSyndicationConfig` - Syndication configuration is invalid

## Best Practices

1. **Member Selection**: Carefully select syndicate members with good credit history and reliable repayment behavior

2. **Share Distribution**: Ensure share percentages reflect actual contribution and risk tolerance

3. **Collateral Requirements**: Maintain adequate collateral to cover potential defaults

4. **Approval Process**: Require high approval percentage (75%+) to ensure consensus

5. **Communication**: Maintain clear communication among syndicate members

6. **Legal Agreements**: Consider off-chain legal agreements to supplement on-chain enforcement

7. **Monitoring**: Regularly monitor syndication status and loan repayment progress

8. **Exit Strategy**: Plan for member exits and syndication cancellation scenarios

## Integration with Existing Systems

The syndication system integrates with:

- **Loan System**: Syndications create standard loans with co-borrowers and guarantors
- **Vouching System**: Members contribute vouches to strengthen syndication
- **Credit Score System**: Syndication performance affects members' credit scores
- **Default Handling**: Syndication defaults trigger collateral slashing from all members
- **Admin Functions**: Syndication configuration requires admin approval

## Security Considerations

1. **Lead Borrower Control**: Only lead borrower can cancel syndication before disbursement

2. **Approval Thresholds**: High approval percentage required to prevent unilateral decisions

3. **Collateral Protection**: Collateral is only slashed after default and grace period

4. **Member Limits**: Maximum and minimum member limits prevent abuse

5. **Share Validation**: Total shares must equal 100% to prevent over-allocation

6. **Status Checks**: All operations validate syndication status before proceeding

## Performance Considerations

- Syndication creation is O(1) complexity
- Member joining is O(n) where n is current member count
- Loan disbursement is O(n) for co-borrower setup
- Repayment is O(1) for individual contributions
- Default handling is O(n) for collateral slashing

Storage cost per syndication: ~500 bytes + ~200 bytes per member

## Example Use Cases

### Use Case 1: Small Business Partnership

Three business partners create a syndication to fund a joint venture:
- Partner A (Lead Borrower): 50% share, 50% collateral
- Partner B (Co-Borrower): 30% share, 30% collateral
- Partner C (Co-Borrower): 20% share, 20% collateral

### Use Case 2: Family Loan

Family members pool together for a large purchase:
- Parent (Lead Borrower): 60% share, 60% collateral
- Child 1 (Co-Borrower): 20% share, 20% collateral
- Child 2 (Co-Borrower): 20% share, 20% collateral

### Use Case 3: Community Project

Community members fund a shared project:
- Project Lead (Lead Borrower): 40% share, 40% collateral
- Community Member 1 (Co-Borrower): 30% share, 30% collateral
- Community Member 2 (Co-Borrower): 30% share, 30% collateral
- External Guarantor (Guarantor): 0% share, 20% additional collateral

## Future Enhancements

Potential future improvements:

- Dynamic share rebalancing
- Partial member exits with collateral return
- Syndication voting for major decisions
- Cross-chain syndication support
- Syndication insurance
- Automated repayment scheduling
- Syndication marketplace for member matching
- Reputation system for syndicate members
- Syndication templates for common use cases

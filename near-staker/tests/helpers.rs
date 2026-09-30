use near_sdk::test_utils::accounts;
use near_sdk::{
    json_types::{U128, U64},
    near, AccountId, Gas, NearToken, PublicKey,
};
use near_workspaces::result::ExecutionFinalResult;
use near_workspaces::{network::Sandbox, Account, Contract, Worker};
use serde_json::json;

use uint::construct_uint;

#[path = "types.rs"]
mod types;
use types::StakerInfo;

construct_uint! {
    pub struct U256(4);
}

#[near(serializers = [json, borsh])]
pub struct RewardFeeFraction {
    pub numerator: u32,
    pub denominator: u32,
}

pub const ONE_NEAR: u128 = 10_u128.pow(24);
pub const TWENTY_NEAR: NearToken = NearToken::from_near(20);

#[macro_export]
// A macro to check that two values are equal or within a difference of an epsilon
macro_rules! assert_approx_eq {
    ($left:expr, $right:expr, $epsilon:expr) => {{
        let left_val = $left;
        let right_val = $right;
        let epsilon_val = $epsilon;
        if (left_val > right_val && left_val - right_val > epsilon_val)
            || (right_val > left_val && right_val - left_val > epsilon_val)
        {
            panic!(
                "assertion failed: `(left ≈ right)` \
                 (left: `{:?}`, right: `{:?}`, epsilon: `{:?}`)",
                left_val, right_val, epsilon_val
            );
        }
    }};
}

pub async fn setup() -> Result<(Account, Worker<Sandbox>, Contract), Box<dyn std::error::Error>> {
    // Setup sandbox
    let sandbox = near_workspaces::sandbox().await?;
    // Compile the contract
    let contract_wasm = near_workspaces::compile_project("./").await?;
    // Get the owner account
    let owner = sandbox.root_account()?;
    // Deploy the contract to the sandbox from the owner
    let contract = owner.deploy(&contract_wasm).await?.unwrap();
    // Return the owner, sandbox, and contract
    Ok((owner, sandbox, contract))
}

pub async fn setup_contract(
) -> Result<(Account, Worker<Sandbox>, Contract), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup().await?;

    // Initialize the contract
    let init = contract
        .call("new")
        .args_json(json!({
            "owner_id": owner.id(),
            "treasury": accounts(1),
            "default_delegation_pool": accounts(2),
        }))
        .transact()
        .await?;
    assert!(init.is_success());

    Ok((owner, sandbox, contract))
}

pub async fn setup_contract_with_code(
    code_path: String,
) -> Result<(Account, Worker<Sandbox>, Contract), Box<dyn std::error::Error>> {
    // deploy the code provided at the path
    let contract_wasm = std::fs::read(code_path)?;
    let sandbox = near_workspaces::sandbox().await?;
    let owner = sandbox.root_account()?;
    let contract = owner.deploy(&contract_wasm).await?.unwrap();

    let default_pool = setup_pool(&sandbox, &owner, "default-pool").await?;

    // init the contract
    let init = contract
        .call("new")
        .args_json(json!({
            "owner_id": owner.id(),
            "treasury": accounts(1),
            "default_delegation_pool": default_pool.id(),
        }))
        .transact()
        .await?;
    assert!(init.is_success());
    Ok((owner, sandbox, contract))
}

pub async fn setup_pool(
    worker: &Worker<Sandbox>,
    owner: &Account,
    name: &str,
) -> Result<Contract, Box<dyn std::error::Error>> {
    let pool_wasm = std::fs::read("./tests/external-contracts/twinstake_poolv1_near.wasm")?;
    let contract = worker.dev_deploy(&pool_wasm).await?;

    let deployer = owner
        .create_subaccount(name)
        .initial_balance(NearToken::from_near(3000))
        .transact()
        .await?
        .unwrap();

    let key: PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
        .parse()
        .unwrap();
    let pool_init = deployer
        .call(contract.id(), "new")
        .args_json(json!({
            "owner_id": owner.id(),
            "stake_public_key": key,
            "reward_fee_fraction": RewardFeeFraction { numerator: 10, denominator: 100 },
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(pool_init.is_success());

    let first_stake = deployer
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(1000))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(first_stake.is_success());

    Ok(contract)
}

pub async fn setup_breakable_pool(
    worker: &Worker<Sandbox>,
    owner: &Account,
    name: &str,
) -> Result<Contract, Box<dyn std::error::Error>> {
    let pool_wasm = std::fs::read("./tests/external-contracts/breakable_pool.wasm")?;
    let contract = worker.dev_deploy(&pool_wasm).await?;

    let deployer = owner
        .create_subaccount(name)
        .initial_balance(NearToken::from_near(3000))
        .transact()
        .await?
        .unwrap();

    let key: PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
        .parse()
        .unwrap();
    let pool_init = deployer
        .call(contract.id(), "new")
        .args_json(json!({
            "owner_id": owner.id(),
            "stake_public_key": key,
            "reward_fee_fraction": RewardFeeFraction { numerator: 10, denominator: 100 },
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(pool_init.is_success());

    let first_stake = deployer
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(1000))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(first_stake.is_success());

    Ok(contract)
}

pub async fn setup_contract_with_pool(
) -> Result<(Account, Worker<Sandbox>, Contract, Contract), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup().await?;

    let default_pool = setup_pool(&sandbox, &owner, "default-pool").await?;

    // Initialize the contract
    let init = contract
        .call("new")
        .args_json(json!({
            "owner_id": owner.id(),
            "treasury": accounts(1),
            "default_delegation_pool": default_pool.id(),
        }))
        .transact()
        .await?;
    assert!(init.is_success());

    Ok((owner, sandbox, contract, default_pool))
}

pub async fn setup_user(
    worker: &Worker<Sandbox>,
    account_id: &str,
) -> Result<Account, Box<dyn std::error::Error>> {
    setup_user_with_tokens(worker, account_id, 20).await
}

pub async fn setup_user_with_tokens(
    worker: &Worker<Sandbox>,
    account_id: &str,
    near_balance: u128,
) -> Result<Account, Box<dyn std::error::Error>> {
    let account = worker.dev_create_account().await?;
    let user = account
        .create_subaccount(account_id)
        .initial_balance(NearToken::from_near(near_balance))
        .transact()
        .await?
        .unwrap();
    Ok(user)
}

pub async fn setup_whitelisted_user(
    owner: &Account,
    contract: &Contract,
    account_id: &str,
) -> Result<Account, Box<dyn std::error::Error>> {
    let user = owner
        .create_subaccount(account_id)
        .initial_balance(TWENTY_NEAR)
        .transact()
        .await?
        .unwrap();

    let whitelist_user = owner
        .call(contract.id(), "add_user_to_whitelist")
        .args_json(json!({
            "user_id": user.id(),
        }))
        .transact()
        .await?;
    assert!(whitelist_user.is_success());
    Ok(user)
}

pub async fn setup_whitelisted_user_with_custom_balance(
    owner: &Account,
    contract: &Contract,
    account_id: &str,
    near_balance: NearToken,
) -> Result<Account, Box<dyn std::error::Error>> {
    let user = owner
        .create_subaccount(account_id)
        .initial_balance(near_balance)
        .transact()
        .await?
        .unwrap();

    let whitelist_user = owner
        .call(contract.id(), "add_user_to_whitelist")
        .args_json(json!({
            "user_id": user.id(),
        }))
        .transact()
        .await?;
    assert!(whitelist_user.is_success());
    Ok(user)
}

pub async fn whitelist_user(
    contract: &Contract,
    owner: &Account,
    account_id: &Account,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = owner
        .call(contract.id(), "add_user_to_whitelist")
        .args_json(json!({
            "user_id": account_id.id(),
        }))
        .transact()
        .await?;

    assert!(result.is_success());

    Ok(())
}

/// For asserting an event was emitted
pub fn get_event(logs: Vec<&str>) -> serde_json::Value {
    let event_logs = logs
        .iter()
        .rev()
        .find(|_log| _log.contains("EVENT_JSON"))
        .unwrap();

    serde_json::from_str(&event_logs["EVENT_JSON:".len()..]).unwrap()
}

// returns all the events included in the logs in the order they were emitted
pub fn get_events(logs: Vec<&str>) -> Vec<serde_json::Value> {
    logs.iter()
        .filter(|_log| _log.contains("EVENT_JSON"))
        .map(|_log| serde_json::from_str(&_log["EVENT_JSON:".len()..]).unwrap())
        .collect::<Vec<serde_json::Value>>()
}

pub fn check_error_msg(response: ExecutionFinalResult, error_message: &str) {
    assert!(response
        .into_result()
        .unwrap_err()
        .to_string()
        .contains(error_message));
}

/// Asserts the concurrency invariant exercised by the mutual-exclusion tests: every concurrently
/// submitted transaction either succeeds or fails for a legitimate concurrency reason — ERR_LOCKED
/// (it lost the `is_locked` race) or ERR_NOT_IN_SYNC (it took the lock but the sandbox epoch advanced
/// mid-test, so the post-lock sync check rejected it). Any other failure is a real bug.
///
/// The number of successes vs. failures is deliberately not asserted. Sandbox scheduling decides
/// whether the transactions truly overlap (one wins, the rest get ERR_LOCKED) or serialize (each runs
/// against an already-released lock and can independently succeed, or independently hit
/// ERR_NOT_IN_SYNC once the epoch drifts), so the split is non-deterministic. The caller separately
/// bounds the resulting `total_staked` change, which is what actually guards against corruption.
pub fn assert_concurrency_outcomes(results: impl IntoIterator<Item = ExecutionFinalResult>) {
    // These mirror the ERR_LOCKED / ERR_NOT_IN_SYNC constants in the contract's errors module. The
    // contract is a cdylib and cannot be imported here, so the strings are duplicated as literals
    // (the same convention the other tests use with check_error_msg).
    const ERR_LOCKED: &str = "Contract is currently executing";
    const ERR_NOT_IN_SYNC: &str = "Contract is not in sync";

    for result in results {
        if result.is_success() {
            continue;
        }
        let err = result.into_result().unwrap_err().to_string();
        assert!(
            err.contains(ERR_LOCKED) || err.contains(ERR_NOT_IN_SYNC),
            "concurrent tx failed for an unexpected reason: {err}"
        );
    }
}

pub async fn get_max_withdraw(
    contract: Contract,
    user: Account,
) -> Result<u128, Box<dyn std::error::Error>> {
    let response = contract
        .view("max_withdraw")
        .args_json(json!({
            "account_id": user.id(),
        }))
        .await?
        .json::<U128>()
        .unwrap();

    println!(">> get_max_withdraw: {}", response.0);

    Ok(response.0)
}

pub async fn get_is_locked(contract: Contract) -> Result<bool, Box<dyn std::error::Error>> {
    let response = contract
        .view("get_is_locked")
        .await?
        .json::<bool>()
        .unwrap();

    Ok(response)
}

pub async fn get_total_staked(
    contract: Contract,
) -> Result<(u128, u64), Box<dyn std::error::Error>> {
    let response = contract
        .view("get_total_staked")
        .args_json(json!({}))
        .await?
        .json::<(U128, U64)>()
        .unwrap();
    println!(
        ">> get_total_staked: {} epoch: {}",
        response.0 .0, response.1 .0
    );
    Ok((response.0 .0, response.1 .0))
}

pub async fn get_tax_exempt_stake(contract: Contract) -> Result<u128, Box<dyn std::error::Error>> {
    let response = contract
        .view("get_tax_exempt_stake")
        .args_json(json!({}))
        .await?
        .json::<U128>()
        .unwrap();
    println!(">> get_tax_exempt_stake: {}", response.0);
    Ok(response.0)
}

pub async fn get_trunear_balance(
    contract: &Contract,
    user: &AccountId,
) -> Result<u128, Box<dyn std::error::Error>> {
    let response = contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": user
        }))
        .await?
        .json::<U128>()
        .unwrap();

    Ok(response.0)
}

pub async fn get_share_price(contract: Contract) -> Result<u128, Box<dyn std::error::Error>> {
    let response = contract
        .view("share_price")
        .args_json(json!({}))
        .await?
        .json::<(String, String)>()
        .unwrap();

    let num = U256::from_dec_str(&response.0).unwrap();
    let denom = U256::from_dec_str(&response.1).unwrap();
    let share_price = (num / denom).as_u128();

    println!(">> get_share_price: {}", share_price);
    Ok(share_price)
}

pub async fn get_ft_price(contract: Contract) -> Result<U128, Box<dyn std::error::Error>> {
    let response = contract
        .view("ft_price")
        .args_json(json!({}))
        .await?
        .json::<U128>()
        .unwrap();

    Ok(response)
}

pub async fn share_price_fraction(
    contract: &Contract,
) -> Result<(U256, U256), Box<dyn std::error::Error>> {
    let response = contract
        .view("share_price")
        .args_json(json!({}))
        .await?
        .json::<(String, String)>()
        .unwrap();

    let num = U256::from_dec_str(&response.0).unwrap();
    let denom = U256::from_dec_str(&response.1).unwrap();

    Ok((num, denom))
}

pub async fn update_total_staked(
    contract: Contract,
    owner: Account,
) -> Result<ExecutionFinalResult, Box<dyn std::error::Error>> {
    let update_total_staked = owner
        .call(contract.id(), "update_total_staked")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(update_total_staked.is_success());

    Ok(update_total_staked)
}

pub async fn move_epoch_forward(
    sandbox: &Worker<Sandbox>,
    contract: &Contract,
) -> Result<(), Box<dyn std::error::Error>> {
    let initial_epoch = get_current_epoch(contract).await?;
    loop {
        sandbox.fast_forward(10).await.unwrap();
        let current_epoch = get_current_epoch(contract).await?;
        if current_epoch > initial_epoch {
            break;
        }
    }

    Ok(())
}

pub async fn move_epoch_forward_and_update_total_staked(
    sandbox: &Worker<Sandbox>,
    contract: &Contract,
    owner: Account,
) -> Result<(), Box<dyn std::error::Error>> {
    move_epoch_forward(sandbox, contract).await?;

    let _ = update_total_staked(contract.clone(), owner.clone()).await?;

    Ok(())
}

pub async fn get_account_staked_balance(
    contract: &Contract,
    account_id: AccountId,
) -> Result<u128, Box<dyn std::error::Error>> {
    let response = contract
        .view("get_account_staked_balance")
        .args_json(json!({
            "account_id": account_id,
        }))
        .await?
        .json::<U128>()
        .unwrap();
    println!(">> get_account_staked_balance: {}", response.0);
    Ok(response.0)
}

pub async fn get_account_unstaked_balance(
    contract: &Contract,
    account_id: AccountId,
) -> Result<u128, Box<dyn std::error::Error>> {
    let response = contract
        .view("get_account_unstaked_balance")
        .args_json(json!({
            "account_id": account_id,
        }))
        .await?
        .json::<U128>()
        .unwrap();
    println!(">> get_account_unstaked_balance: {}", response.0);
    Ok(response.0)
}

pub async fn get_current_epoch(contract: &Contract) -> Result<u64, Box<dyn std::error::Error>> {
    let response = contract
        .view("get_staker_info")
        .await?
        .json::<StakerInfo>()
        .unwrap();

    Ok(response.current_epoch.0)
}

pub async fn get_treasury_id(contract: &Contract) -> Result<AccountId, Box<dyn std::error::Error>> {
    let response = contract
        .view("get_staker_info")
        .await?
        .json::<StakerInfo>()
        .unwrap();

    Ok(response.treasury_id)
}

pub async fn get_total_supply(contract: &Contract) -> Result<u128, Box<dyn std::error::Error>> {
    let response = contract
        .view("ft_total_supply")
        .args_json(json!({}))
        .await?;
    let total_supply = response.json::<U128>().unwrap();

    Ok(total_supply.0)
}

pub async fn get_latest_unstake_nonce(
    contract: &Contract,
) -> Result<u128, Box<dyn std::error::Error>> {
    let response = contract
        .view("get_latest_unstake_nonce")
        .await?
        .json::<U128>()
        .unwrap();
    println!(">> get_latest_unstake_nonce: {}", response.0);
    Ok(response.0)
}

pub async fn stake(
    contract: &Contract,
    user: Account,
    amount: u128,
) -> Result<ExecutionFinalResult, Box<dyn std::error::Error>> {
    let stake = user
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(amount))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    Ok(stake)
}

pub async fn stake_to_specific_pool(
    contract: &Contract,
    user: Account,
    pool_id: AccountId,
    amount: u128,
) -> Result<ExecutionFinalResult, Box<dyn std::error::Error>> {
    let stake = user
        .call(contract.id(), "stake_to_specific_pool")
        .args_json(json!({
            "pool_id": pool_id,
        }))
        .deposit(NearToken::from_near(amount))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    Ok(stake)
}

pub async fn unstake(
    contract: &Contract,
    user: Account,
    amount: u128,
) -> Result<ExecutionFinalResult, Box<dyn std::error::Error>> {
    let unstake = user
        .call(contract.id(), "unstake")
        .args_json(json!(
            {"amount": U128::from(amount * ONE_NEAR)}
        ))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    Ok(unstake)
}

pub async fn increase_total_staked(
    contract: &Contract,
    owner: &Account,
    user_name: &str,
    amount: u128,
) -> Result<ExecutionFinalResult, Box<dyn std::error::Error>> {
    let user = owner
        .create_subaccount(user_name)
        .initial_balance(NearToken::from_near(amount + 1))
        .transact()
        .await?
        .unwrap();
    whitelist_user(contract, owner, &user).await?;

    let stake = user
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(amount))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    Ok(stake)
}

pub async fn set_fee(
    contract: &Contract,
    owner: &Account,
    amount: u128,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = owner
        .call(contract.id(), "set_fee")
        .args_json(json!({
            "new_fee": amount,
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    Ok(())
}

pub async fn set_min_deposit(
    contract: &Contract,
    owner: &Account,
    amount: u128,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = owner
        .call(contract.id(), "set_min_deposit")
        .args_json(json!({
            "min_deposit": NearToken::from_near(amount),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    Ok(())
}

pub async fn register_account(
    contract: &Contract,
    caller: &Account,
    account: &AccountId,
) -> Result<(), Box<dyn std::error::Error>> {
    let register = caller
        .call(contract.id(), "storage_deposit")
        .args_json(json!(
            {
                "account_id": account.clone(),
                "registration_only": true
            }
        ))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(register.is_success());

    Ok(())
}

pub async fn transfer_trunear(
    contract: &Contract,
    sender: &Account,
    recipient: &AccountId,
    amount: u128,
) -> Result<(), Box<dyn std::error::Error>> {
    let transfer = sender
        .call(contract.id(), "ft_transfer")
        .args_json(json!({
            "receiver_id": recipient.clone(),
            "amount": U128::from(amount),
        }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;
    assert!(transfer.is_success());

    Ok(())
}

pub fn mul_div_with_rounding(x: U256, y: U256, denominator: U256, rounding_up: bool) -> U256 {
    let mut result = x * y / denominator;
    let remainder = (x * y) % denominator;
    if rounding_up && !remainder.is_zero() {
        result += U256::from(1)
    }
    result
}

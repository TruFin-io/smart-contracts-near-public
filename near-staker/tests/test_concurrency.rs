use near_sdk::json_types::U128;
use near_sdk::{Gas, NearToken};
use serde_json::json;
pub mod constants;
pub mod helpers;
use helpers::*;

pub mod event;
mod types;
use tokio::try_join;

#[tokio::test]
async fn test_simultaneous_stake_unstake_yields_constant_total_staked(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;

    // Ensure contract is in sync right before staking (epoch may have advanced during setup)
    let _ = update_total_staked(contract.clone(), owner.clone()).await?;

    // alice stakes 10 NEAR
    let alice_stake_amount = 10;
    let stake: near_workspaces::result::ExecutionFinalResult =
        stake(&contract, alice.clone(), alice_stake_amount).await?;
    assert!(stake.is_success());

    // bob stakes 10 NEAR
    let stake = bob
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    // alice and bob stake and unstake respectively 2 NEAR 4 times, for a total of 8 NEAR
    let unstakes_count = 4;
    for _ in 0..unstakes_count {
        // capture total_staked per iteration: the winner changes it, so a single pre-loop
        // snapshot would drift cumulatively across iterations.
        let (pre_total_staked, _) = get_total_staked(contract.clone()).await?;

        let bob_deposit_tx = bob
            .call(contract.id(), "stake")
            .deposit(NearToken::from_near(2))
            .gas(Gas::from_tgas(300))
            .transact();

        let alice_unstake_tx = alice
            .call(contract.id(), "unstake")
            .args_json(json!({
                "amount": U128::from(2 * ONE_NEAR),
            }))
            .deposit(NearToken::from_near(3))
            .gas(Gas::from_tgas(300))
            .transact();

        let (bob_deposit_result, alice_unstake_result) =
            try_join!(bob_deposit_tx, alice_unstake_tx)?;

        // Each tx either wins the is_locked race or is rejected for a concurrency/sync reason; the
        // split is non-deterministic in the sandbox, so we assert only that no tx fails unexpectedly.
        assert_concurrency_outcomes([bob_deposit_result, alice_unstake_result]);

        let (total_staked, _) = get_total_staked(contract.clone()).await?;

        // total_staked moves by at most 2 NEAR per iteration: a lone stake or unstake shifts it by 2,
        // a stake and unstake that both land cancel out, and locked/out-of-sync rejections change nothing.
        assert!(pre_total_staked.abs_diff(total_staked) <= NearToken::from_near(2).as_yoctonear());
    }

    Ok(())
}

#[tokio::test]
async fn test_simultaneous_stake_unstake_and_update_total_staked_results_in_nondeterministic_total_staked(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;

    // Ensure contract is in sync right before staking (epoch may have advanced during setup)
    let _ = update_total_staked(contract.clone(), owner.clone()).await?;

    // alice stakes 10 NEAR
    let alice_stake_amount = 10;
    let stake: near_workspaces::result::ExecutionFinalResult =
        stake(&contract, alice.clone(), alice_stake_amount).await?;
    assert!(stake.is_success());

    // bob stakes 10 NEAR
    let stake = bob
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    // alice and bob simultaneously stake and unstake 2 NEAR 4 times, for a total of 8 NEAR
    let unstakes_count = 4;

    for _ in 0..unstakes_count {
        // capture total_staked per iteration: the winner changes it, so a single pre-loop
        // snapshot would drift cumulatively across iterations.
        let (pre_total_staked, _) = get_total_staked(contract.clone()).await?;

        let bob_deposit_tx = bob
            .call(contract.id(), "stake")
            .deposit(NearToken::from_near(2))
            .gas(Gas::from_tgas(300))
            .transact();

        let update_total_staked = owner
            .call(contract.id(), "update_total_staked")
            .gas(Gas::from_tgas(300))
            .transact();

        let alice_unstake_tx = alice
            .call(contract.id(), "unstake")
            .args_json(json!({
                "amount": U128::from(2 * ONE_NEAR),
            }))
            .deposit(NearToken::from_near(3))
            .gas(Gas::from_tgas(300))
            .transact();

        let (alice_unstake_result, update_total_staked, bob_deposit_result) =
            try_join!(alice_unstake_tx, update_total_staked, bob_deposit_tx)?;

        // Each tx either wins the is_locked race or is rejected for a concurrency/sync reason; the
        // split is non-deterministic in the sandbox, so we assert only that no tx fails unexpectedly.
        assert_concurrency_outcomes([
            alice_unstake_result,
            update_total_staked,
            bob_deposit_result,
        ]);

        let (total_staked, _) = get_total_staked(contract.clone()).await?;

        // total_staked moves by at most 2 NEAR per iteration: a lone stake or unstake shifts it by 2,
        // a stake and unstake that both land cancel out, and update_total_staked only reconciles to the
        // pool's real balance (locked/out-of-sync rejections change nothing).
        assert!(pre_total_staked.abs_diff(total_staked) <= NearToken::from_near(2).as_yoctonear());
    }

    // depending on the non-deterministic order of the transactions, the share price may or may not change.

    Ok(())
}

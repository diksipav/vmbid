use actix_rt;
use project::handlers::{handle_buy, handle_sell};
use project::state::AppState;
use proptest::prelude::*;
use proptest::sample::select;

// TODO: Hmm I know that proptest-regretions should be saved and commited, but
// the initial failure was due to the wrong test, so not sure should I delete
// it? I ended up deleting which maybe is not the best because other team
// members will run tests with diff seeds?

// TODO: Does the order really matter here? In the book it
// says it's better to go from simplest to more complicated
// because shrinking is done toward items earlier in the list.
#[derive(Debug)]
pub enum Action {
    Sell(u64),
    Buy(String, u64, u64),
}

// Strategy to generate random buy and sell actions
fn action_strategy() -> impl Strategy<Value = Action> {
    prop_oneof![
        (0u64..250).prop_map(|vol| Action::Sell(vol)),
        (".{0,6}", 0u64..20, 0u64..250)
            .prop_map(|(user, price, vol)| Action::Buy(user, price, vol)),
    ]
}

// Strategy to generate random buy and sell actions using only few users,
// used in test for monotone allocations invariant.
fn action_strategy_few_users() -> impl Strategy<Value = Action> {
    prop_oneof![
        (select(vec![50u64, 100, 150, 200, 250])).prop_map(|vol| Action::Sell(vol)),
        (
            select(vec!["u1".to_string(), "u2".to_string(), "u3".to_string()]),
            1u64..10,
            select(vec![50u64, 100, 150, 200, 250])
        )
            .prop_map(|(user, price, vol)| Action::Buy(user, price, vol)),
    ]
}

// Calculates total sold and total bought volume in the system
fn total_volume_in_the_system(state: &AppState) -> (u64, u64) {
    let allocations: u64 = state.state.allocations.lock().unwrap().values().sum();

    let supply = *state.state.supply.lock().unwrap();

    let open_bids: u64 = state
        .state
        .bids
        .lock()
        .unwrap()
        .values()
        .flat_map(|queue| queue.iter())
        .map(|bid| bid.volume)
        .sum();

    (allocations + supply, allocations + open_bids)
}

// TODO: I'm not sure what is the most idiomatic here to do, to use
// tokio runtime or to use actix_rt like I did leaving the test sync?
// Or #[actix_rt::test] is needed? Or to use the proptest_async crate?
proptest! {
    #[test]
    fn test_volume_conservation(
        actions in prop::collection::vec(action_strategy(), 1..20)
    ) {
          actix_rt::System::new().block_on(async {
            let state = AppState::default();
            let mut total_bought = 0u64;
            let mut total_sold = 0u64;

            for action in actions {
                match action {
                    Action::Sell(volume) => {
                        handle_sell(&state, volume).await;
                        total_sold += volume;
                    },
                    Action::Buy(username, price, volume) => {
                        if handle_buy(&state, username, price, volume).await.is_ok() {
                            total_bought += volume;
                        }
                    }

                }
            }

            let (sold_in_system, bought_in_system) = total_volume_in_the_system(&state);

            prop_assert_eq!(sold_in_system, total_sold,
                "Volume not conserved! Sold in the system: {}, Total sold: {}",
                sold_in_system, total_sold
            );

            prop_assert_eq!(bought_in_system, total_bought,
                "Volume not conserved! Bought in the system: {}, Total bought: {})",
                bought_in_system, total_bought
            );

            Ok(())
        }).unwrap();
    }



    #[test]
    fn test_allocations_never_decrease(
        actions in prop::collection::vec(action_strategy_few_users(), 1..20)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let state = AppState::default();
            let mut previous_allocations: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

            for action in actions {
                match action {
                    Action::Sell(volume) => {
                        handle_sell(&state, volume).await;
                    },
                    Action::Buy(username, price, volume) => {
                        let _ = handle_buy(&state, username.clone(), price, volume).await;

                        // Check allocation didn't decrease
                        let allocations = state.state.allocations.lock().unwrap();
                        if let Some(&current) = allocations.get(&username) {
                            let previous = previous_allocations.get(&username).copied().unwrap_or(0);
                            prop_assert!(current >= previous,
                                "Allocation decreased for {}: {} -> {}", username, previous, current);
                            previous_allocations.insert(username, current);
                        }
                    }
                }
            }

            Ok(())
        }).unwrap();
    }
}

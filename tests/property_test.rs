// TODO: how idiomatic is to import things with use inside functions/tests?
use std::collections::HashMap;

use project::{
    handlers::{handle_buy, handle_sell},
    state::AppState,
};
use proptest::{prelude::*, sample::select};
use tokio::runtime::Runtime;

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
        (".{0,6}", 0u64..250, 0u64..20)
            .prop_map(|(user, vol, price)| Action::Buy(user, vol, price)),
    ]
}

// Strategy to generate random buy and sell actions using only few users,
// used in test for monotone allocations invariant.
fn action_strategy_few_users() -> impl Strategy<Value = Action> {
    prop_oneof![
        (select(vec![50u64, 100, 150, 200, 250])).prop_map(|vol| Action::Sell(vol)),
        (
            select(vec!["u1".to_string(), "u2".to_string(), "u3".to_string()]),
            select(vec![50u64, 100, 150, 200, 250]),
            1u64..10,
        )
            .prop_map(|(user, vol, price)| Action::Buy(user, vol, price)),
    ]
}

// TODO: I'm not sure what is the most idiomatic here, to use
// tokio runtime or to use tokio::rt leaving the test sync?
// Or to use the proptest_async crate? I believe my approach
// is the best since async crate is still not mature enough.
proptest! {
    #[test]
    // INVARIANTs:
    // total bought == bids + allocations
    // total sold == current supply + allocations
    fn test_volume_conservation(
        actions in prop::collection::vec(action_strategy(), 1..20)
    ) {
      let rt = Runtime::new()?;
      rt.block_on(async {
          let state = AppState::default();
          let mut total_bought = 0u64;
          let mut total_sold = 0u64;

          for action in actions {
              match action {
                  Action::Sell(volume) => {
                      handle_sell(&state, volume).await;
                      total_sold += volume;
                  },
                  Action::Buy(username, volume, price) => {
                      if handle_buy(&state, username, volume, price).await.is_ok() {
                          total_bought += volume;
                      }
                  }

              }
          }

          let (sold_in_system, bought_in_system) = state.total_volume_in_the_system();

          prop_assert_eq!(sold_in_system, total_sold,
              "Volume not conserved! Sold in the system: {}, Total sold: {}",
              sold_in_system, total_sold
          );

          prop_assert_eq!(bought_in_system, total_bought,
              "Volume not conserved! Bought in the system: {}, Total bought: {})",
              bought_in_system, total_bought
          );

          Ok(())
      })?;
    }

    #[test]
    // Allocations per user never decrease
    fn test_allocations_never_decrease(
        actions in prop::collection::vec(action_strategy_few_users(), 1..20)
    ) {
      let rt = Runtime::new()?;

      rt.block_on(async {
          let state = AppState::default();
          let mut prev_allocations: HashMap<String, u64> = HashMap::new();

          for action in actions {
              match action {
                  Action::Sell(volume) => {
                      handle_sell(&state, volume).await;
                  },
                  Action::Buy(username, volume, price) => {
                      let _ = handle_buy(&state, username.clone(), volume, price).await;

                      // Check allocation didn't decrease
                      let allocations = state.allocations.lock();
                      if let Some(&current) = allocations.get(&username) {
                          let previous = prev_allocations.get(&username).copied().unwrap_or(0);
                          prop_assert!(current >= previous,
                              "Allocation decreased for {}: {} -> {}", username, previous, current);
                          prev_allocations.insert(username, current);
                      }
                  }
              }
          }

          Ok(())
      })?;
    }
}

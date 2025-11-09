use actix_web::rt;
use project::handlers::{handle_buy, handle_sell};
use project::state::AppState;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[actix_web::test]
async fn test_concurrent_buys_maintain_fifo() {
    let state = Arc::new(AppState::default());

    // Spawn 100 concurrent buy tasks
    let mut handles = vec![];
    for i in 0..100 {
        // TODO: is it more idiomatic to use shadowing here: let state = ...
        let state_clone = state.clone();
        let handle = rt::spawn(async move {
            handle_buy(
                &state_clone,
                format!("user{}", i),
                5, // Same price - tests FIFO
                10,
            )
            .await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Assert: All bids created, seq numbers are unique and sequential
    let bids = state.state.bids.lock().unwrap();
    let queue = bids.get(&5).unwrap();

    assert_eq!(queue.len(), 100, "All bids should be created");

    let seq_numbers: Vec<u64> = queue.iter().map(|b| b.seq).collect();

    for (i, &seq) in seq_numbers.iter().enumerate() {
        assert_eq!(seq, i as u64, "Seq numbers should be 0..99");
    }
}

#[actix_web::test]
// INVARIANTs:
// total bought == bids + allocations
// total sold == current supply + allocations
async fn test_concurrent_buy_and_sell_conservation() {
    let state = Arc::new(AppState::default());

    // Spawn 100 concurrent buy tasks and 100 sell tasks
    let mut buy_handles = vec![];
    let mut sell_handles = vec![];

    let mut total_bought = 0;
    let mut total_sold = 0;
    for i in 0..100 {
        let state_clone = state.clone();
        let handle = rt::spawn(async move {
            handle_buy(&state_clone, format!("user{}", i), i % 10, 50 * (i % 10)).await
        });
        buy_handles.push(handle);
        total_bought += 50 * (i % 10);

        let state_clone = state.clone();
        let handle = rt::spawn(async move { handle_sell(&state_clone, 350).await });
        sell_handles.push(handle);
        total_sold += 350;
    }

    // Wait for all buy tasks to complete
    for handle in buy_handles {
        handle.await.unwrap().unwrap();
    }

    // Wait for all sell tasks  to complete
    for handle in sell_handles {
        handle.await.unwrap();
    }

    let (sold_in_system, bought_in_system) = state.total_volume_in_the_system();

    // INVARIANT: total bought == bids + allocations
    assert_eq!(
        bought_in_system, total_bought,
        "Volume not conserved! Bought in the system: {}, Total bought: {})",
        bought_in_system, total_bought
    );

    // INVARIANT: total sold == current supply + allocations
    assert_eq!(
        sold_in_system, total_sold,
        "Volume not conserved! Sold in the system: {}, Total sold: {}",
        sold_in_system, total_sold
    );
}

#[actix_web::test]
async fn test_concurrent_allocations_never_decrease() {
    let state = Arc::new(AppState::default());
    let alloc_history: Arc<Mutex<HashMap<String, Vec<u64>>>> = Arc::new(Mutex::new(HashMap::new()));

    // Spawn 100 concurrent buy and 100 sell tasks. Use random
    // delay to make tasks call buy and sell handlers at ranom times.
    let mut buy_handles = vec![];
    let mut sell_handles = vec![];

    for i in 0..100 {
        let state_clone = state.clone();
        buy_handles.push(rt::spawn(async move {
            let delay = rand::rng().random_range(0..20);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

            let username = format!("user{}", i % 3);
            handle_buy(&state_clone, username.clone(), i % 10, 50 * (i % 10))
                .await
                .unwrap();
        }));

        let state_clone = state.clone();
        sell_handles.push(rt::spawn(async move {
            let delay = rand::rng().random_range(0..20);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

            handle_sell(&state_clone, 250).await
        }));
    }

    // Sample allocations periodically during execution
    let state_clone = state.clone();
    let history_clone = alloc_history.clone();
    let monitor = rt::spawn(async move {
        for _ in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

            let allocs = state_clone.state.allocations.lock().unwrap();
            let mut history = history_clone.lock().unwrap();
            println!("history {:?}", history);
            for (user, &allocation) in allocs.iter() {
                history
                    .entry(user.clone())
                    .or_insert_with(Vec::new)
                    .push(allocation);
            }
        }
    });

    // Wait for all buy tasks to complete
    for handle in buy_handles {
        handle.await.unwrap();
    }

    // Wait for all sell tasks to complete
    for handle in sell_handles {
        handle.await.unwrap();
    }

    monitor.await.unwrap();

    // Check history: allocations should never decrease
    let history = alloc_history.lock().unwrap();
    for (user, allocations) in history.iter() {
        for window in allocations.windows(2) {
            assert!(
                window[1] >= window[0],
                "Allocation decreased for {}: {} -> {}",
                user,
                window[0],
                window[1]
            );
        }
    }
}

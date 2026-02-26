use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use project::{
    handlers::{handle_buy, handle_sell},
    state::AppState,
};
use rand::Rng;

#[tokio::test]
async fn test_concurrent_buys_maintain_fifo() {
    let state = Arc::new(AppState::default());

    // Spawn 100 concurrent buy tasks
    let mut handles = vec![];
    for i in 0..100 {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            handle_buy(
                &state_clone,
                format!("user{}", i),
                10,
                5, // Same price - tests FIFO
            )
            .await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Assert: All bids created, seq numbers are unique
    let bids = state.bids.lock();
    let queue = bids.get(&5).unwrap();

    assert_eq!(queue.len(), 100, "All bids should be created");

    let mut seq_numbers: Vec<u64> = queue.iter().map(|b| b.seq).collect();
    seq_numbers.sort();
    // Seq numbers should be unique and cover 0..99
    assert_eq!(
        seq_numbers,
        (0u64..100).collect::<Vec<_>>(),
        "Seq numbers should be unique and sequential"
    );
}

#[tokio::test]
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
        let handle = tokio::spawn(async move {
            handle_buy(&state_clone, format!("user{}", i), 50 * (i % 10), i % 10).await
        });
        buy_handles.push(handle);
        total_bought += 50 * (i % 10);

        let state_clone = state.clone();
        let handle = tokio::spawn(async move { handle_sell(&state_clone, 350).await });
        sell_handles.push(handle);
        total_sold += 350;
    }

    // Wait for all buy tasks to complete
    for handle in buy_handles {
        handle.await.unwrap().unwrap();
    }

    // Wait for all sell tasks to complete
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

#[tokio::test]
async fn test_concurrent_allocations_never_decrease() {
    let state = Arc::new(AppState::default());
    let alloc_history: Arc<Mutex<HashMap<String, Vec<u64>>>> = Arc::new(Mutex::new(HashMap::new()));

    // Spawn 100 concurrent buy and 100 sell tasks. Use random
    // delay to make tasks call buy and sell handlers at random times.
    let mut buy_handles = vec![];
    let mut sell_handles = vec![];

    for i in 0..100 {
        let state_clone = state.clone();
        buy_handles.push(tokio::spawn(async move {
            let delay = rand::rng().random_range(0..20);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

            let username = format!("user{}", i % 3);
            handle_buy(&state_clone, username.clone(), 50 * (i % 10), i % 10).await.unwrap();
        }));

        let state_clone = state.clone();
        sell_handles.push(tokio::spawn(async move {
            let delay = rand::rng().random_range(0..20);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

            handle_sell(&state_clone, 250).await
        }));
    }

    // Sample allocations periodically during execution
    let state_clone = state.clone();
    let history_clone = alloc_history.clone();
    let monitor = tokio::spawn(async move {
        for _ in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

            let allocs = state_clone.state.allocations.lock();
            let mut history = history_clone.lock().unwrap();
            for (user, &allocation) in allocs.iter() {
                history.entry(user.clone()).or_insert_with(Vec::new).push(allocation);
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

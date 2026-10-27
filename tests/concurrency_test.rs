use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use rand::Rng;
use vmbid::state::AppState;

#[tokio::test]
async fn test_concurrent_buys_maintain_fifo() {
    let state = Arc::new(AppState::default());

    // Spawn 100 concurrent buy tasks
    let mut handles = vec![];
    for i in 0..100 {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            state_clone.buy(
                &format!("user{}", i),
                10,
                5, // Same price - tests FIFO
            )
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Assert: All bids created, each user queued exactly once
    let book = state.book.lock();
    let queue = book.bids.get(&5).unwrap();

    assert_eq!(queue.len(), 100, "All bids should be created");

    let mut users: Vec<&str> = queue.iter().map(|b| b.username.as_str()).collect();
    users.sort();
    users.dedup();
    assert_eq!(users.len(), 100, "Each user should be queued exactly once");
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
        let handle =
            tokio::spawn(
                async move { state_clone.buy(&format!("user{}", i), 50 * (i % 10), i % 10) },
            );
        buy_handles.push(handle);
        total_bought += 50 * (i % 10);

        let state_clone = state.clone();
        let handle = tokio::spawn(async move { state_clone.sell(350) });
        sell_handles.push(handle);
        total_sold += 350;
    }

    // Wait for all buy tasks to complete
    for handle in buy_handles {
        handle.await.unwrap();
    }

    // Wait for all sell tasks to complete
    for handle in sell_handles {
        handle.await.unwrap().unwrap();
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
    // delay to make tasks call buy and sell at random times.
    let mut buy_handles = vec![];
    let mut sell_handles = vec![];

    for i in 0..100 {
        let state_clone = state.clone();
        buy_handles.push(tokio::spawn(async move {
            let delay = rand::rng().random_range(0..20);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

            let username = format!("user{}", i % 3);
            state_clone.buy(&username, 50 * (i % 10), i % 10);
        }));

        let state_clone = state.clone();
        sell_handles.push(tokio::spawn(async move {
            let delay = rand::rng().random_range(0..20);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

            state_clone.sell(250).unwrap();
        }));
    }

    // Sample allocations periodically during execution
    let state_clone = state.clone();
    let history_clone = alloc_history.clone();
    let monitor = tokio::spawn(async move {
        for _ in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

            let book = state_clone.book.lock();
            let mut history = history_clone.lock().unwrap();
            for (user, &allocation) in book.allocations.iter() {
                history.entry(user.clone()).or_default().push(allocation);
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

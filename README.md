# VM-Hours Allocation Service

## Overview

This service implements a VM capacity auction system where:
- Users submit **bids** (username, volume, price) for VM hours through `/buy` endpoints.
- Providers add **supply** through `/sell` endpoints.
- The system automatically **allocates** supply to the highest-priced bids (FIFO within price levels).
- Leftover supply persists and auto-matches future bids.
- It is possible to query the allocation for a specific user through the `/allocation`
endpoint, providing the username as a query parameter.

## Quick Start

### Prerequisites
- Rust 1.78+

### Build and Run

```bash
# Build the project
cargo build

# Run the server (listens on 0.0.0.0:8080)
cargo run

# Run tests
cargo test

# Run specific test suites
cargo test --lib                    # Unit tests
cargo test --test property_test     # Property tests
cargo test --test concurrency_test  # Concurrency tests
```

## API Endpoints

### POST `/buy`
Submit a bid for VM capacity.

**Request:**
```json
{
  "username": "user1",
  "volume": 100,
  "price": 5
}
```

- **Response:** `200 OK`

**Behavior:**
- If leftover supply exists, immediately allocates what's available.
- Remaining volume is queued as a bid.
- Empty username returns `400 Bad Request`.
- Zero volume is accepted (no-op).

### POST `/sell`
Add VM capacity supply to the system.

**Request:**
```json
{
  "volume": 100,
}
```

- **Response:** `200 OK`

**Behavior:**
- Allocates to outstanding bids by price (highest first).
- Within same price level, fills bids in FIFO order.
- Leftover supply is stored for future bids.


### GET `/allocation?username=u1`
Query total allocated VM-hours for a user.

- **Response:** `200 OK with plain text integer body (e.g., "150")`

**Error responses:**
- `400 Bad Request` - Missing username parameter
- `404 Not Found` - Username not found


## Example Usage

```bash
# Start server
cargo run

# In another terminal:

# User1 bids 100 hours at price 3
curl -s -X POST localhost:8080/buy \
  -H 'Content-Type: application/json' \
  -d '{"username":"u1","volume":100,"price":3}'

# User2 bids 150 hours at price 2
curl -s -X POST localhost:8080/buy \
  -H 'Content-Type: application/json' \
  -d '{"username":"u2","volume":150,"price":2}'

# User3 bids 50 hours at price 4 (highest)
curl -s -X POST localhost:8080/buy \
  -H 'Content-Type: application/json' \
  -d '{"username":"u3","volume":50,"price":4}'

# Provider sells 250 hours
curl -s -X POST localhost:8080/sell \
  -H 'Content-Type: application/json' \
  -d '{"volume":250}'

# Check allocations
curl -s 'localhost:8080/allocation?username=u1'  # Returns: 100
curl -s 'localhost:8080/allocation?username=u2'  # Returns: 100 (50 still open)
curl -s 'localhost:8080/allocation?username=u3'  # Returns: 50
```

## Architecture

### Core Components

**AppState:**
- `bids: Mutex<BTreeMap<u64, BinaryHeap<Bid>>>` - Open bids organized by price
- `supply: Mutex<u64>` - Leftover supply from sells
- `allocations: Mutex<HashMap<String, u64>>` - Total allocated per user
- `seq: AtomicU64` - Monotonic sequence for FIFO ordering

**Allocation Algorithm:**
1. **Buy:** Check leftover supply first, allocate immediately if available, queue remainder as bid
2. **Sell:** Iterate bids from highest to lowest price, fill in FIFO order (via seq), store leftovers in supply
3. **FIFO guarantee:** Atomic sequence counter ensures deterministic ordering under concurrency

## Design Rationale

### Data Structures

I chose `BTreeMap<u64, BinaryHeap<Bid>>` for bids because:
- BTreeMap is an ordered map, and we need to sort by prices (FIFO inside a price level) so BTreeMap seems like a reasonable DS for this.
- BinaryHeap for the priority queue, we need so sort queues by sequence. I firstly used VecDeque (first thing that came to my mind when I read FIFO), but then while coding I realised that /buy requests can arrive in different order than the order they acquire the lock. Queue is updated after acquiring the lock so it will not necesarily be ordered by a sequence. BinaryHeap will always sort by the sequence.

I chose `HashMap<String, u64>` for allocations because we want to be able to query users for volume, and the DS does not have to be sorted.

For the sequence I used `AtomicU64` counter, we need a lock-free solution for this in order to be able to serve buyers in the order they submit requests.

### Concurrency Strategy

The system uses coarse-grained locking with three separate mutexes (bids, allocations, supply). I believe this is better than one lock, it creates less contention. There is probably a way to write the project to be lock-free using lock-free data structures and atomic operations but I find it complex for my Rust knowledge level at the moment.

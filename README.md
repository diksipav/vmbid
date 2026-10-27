# Priority auction marketplace for VM compute hours

Bids meet supply, highest price wins.

## Overview

This is a tiny HTTP service that tracks **bids** for VM capacity and **allocates** integer VM-hours when supply arrives:
- Users submit **bids** (username, volume, price) for VM hours through `/buy` endpoints.
- Providers add **supply** through `/sell` endpoints.
- The system automatically **allocates** supply to the highest-priced bids (FIFO within price levels).
- Leftover supply persists and auto-matches future bids.
- It is possible to query the allocation for a specific user through the `/allocation`
endpoint, providing the username as a query parameter.

A **bid** has:
- `username` – unique user ID (string, 2–20 characters)
- `volume` – requested VM-hours (integer > 0)
- `price` – max price **per hour** (integer > 0)

A **supply drop** adds a number of VM-hours to the system.

**Allocation rules**
1. **Highest price wins.**
2. **FIFO inside a price level** (earlier bids at the same price fill first).
3. **Partial fills** allowed; unfilled remainder stays open.
4. **Unused supply** persists and **must auto-match** any *subsequent* bids arriving later (a `/buy` arriving when leftover supply exists should be allocated immediately, no need to wait for the next `/sell`).

## API Endpoints

All errors return JSON: `{"message":"<description>"}`.

### POST `/buy`
- Body: `{"username":"u1","volume":100,"price":3}`
- Responses: **200 OK** with `{"allocated":<u64>,"queued":<u64>}`, or error:
- `422 Unprocessable Entity` - Missing field or alidation failed (username not 2–20 characters, volume or price is 0)
- `400 Bad Request` - Malformed JSON
- `415 Unsupported Media Type` - Missing `Content-Type: application/json`

Behavior:
- Registers bid; **immediately allocate** if leftover supply is available.
- Remaining volume is queued as a bid.

### POST `/sell`
- Body: `{"volume":250}`
- Responses: **200 OK** with `{"allocated":<u64>}`, or error:
- `422 Unprocessable Entity` - Missing field or validation failed (volume is 0) or leftover supply would overflow `u64`
- `400 Bad Request` - Malformed JSON
- `415 Unsupported Media Type` - Missing `Content-Type: application/json`

Behavior:
- Adds supply and allocates to outstanding bids by price (highest first).
- Within same price level, fills bids in FIFO order.
- Leftover supply is stored for future bids.

### GET `/allocation?username=u1`
- Responses: **200 OK** with `{"allocated":<u64>}` (e.g., `{"allocated":150}`), or error:
- `400 Bad Request` - Missing username parameter
- `422 Unprocessable Entity` - Username not 2–20 characters
- `404 Not Found` - Username not found

Behavior:
- Returns the total VM-hours allocated for a user.

## Example

Events:
- t1: u1 bids 100 @ 3
- t2: u2 bids 150 @ 2
- t3: u3 bids 50  @ 4
- t4: provider sells 250

Allocation at t4:
- 50 → u3
- 100 → u1
- 100 → u2 (u2 still open for 50)

## Quick Start

### Prerequisites
- Rust 1.88+

### Build and Run

```bash
# Build the project
cargo build

# Run the server (listens on 0.0.0.0:8080)
cargo run

# Run tests
cargo test

# Run specific test suites
cargo test --test property_test     # Property tests
cargo test --test concurrency_test  # Concurrency tests
```

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
curl -s 'localhost:8080/allocation?username=u1'  # Returns: {"allocated":100}
curl -s 'localhost:8080/allocation?username=u2'  # Returns: {"allocated":100} (50 still open)
curl -s 'localhost:8080/allocation?username=u3'  # Returns: {"allocated":50}
```

## Architecture

**AppState:**
- `book: Mutex<Book>` - All mutable state, behind a single lock:
  - `bids: BTreeMap<u64, VecDeque<Bid>>` - Open bids by price, FIFO within a price level
  - `supply: u64` - Leftover supply from sells
  - `allocations: HashMap<String, u64>` - Total allocated per user

### Data Structures

I chose `BTreeMap<u64, VecDeque<Bid>>` for bids because:
- BTreeMap is an ordered map, and we need to sort by prices so BTreeMap seems like a reasonable DS for this. Iterating it in reverse gives the highest price first.
- VecDeque for the FIFO queue inside a price level (push and pop both O(1)).

I chose `HashMap<String, u64>` for allocations because we want to be able to query users for volume, and the DS does not have to be sorted.

### Concurrency Strategy

All state (bids, supply, allocations) is behind a single mutex. 

The lock is only held for short, synchronous work.


### Future improvements

Research other options besides single mutex and what to do if mutex becomes a bottleneck.

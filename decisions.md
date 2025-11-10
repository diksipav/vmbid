## Decision Log

### 2025-01-05: AppState

I initially wanted to have all the state inside the AppState but couldn't easily implement the Clone trait for it
(I understood that I should not change the initial skeleton). I believe the Arc can also be used to wrap AppState
once the AppState is created. Anyway I ended up wrapping the Inner (actual state) in the Arc inside the AppSTate struct.

### 2025-01-05: Concurrency

I decided to use 3 Mutexes for different parts of the state. I didn't really try with one, it seemed reasonable to me to use separate locks from the beginning in order to have less contention.

### 2025-01-05: Use `Mutex<BTreeMap<u64, VecDeque<Bid>>>` for bids

BTreeMap is an ordered map, and we need to sort by prices (FIFO inside a price level) so BTreeMap seemed like a reasonable DS for this.
I decided to start with VecDeque for the bids queue within the price level because that was my first guess when I read FIFO. I wasn't yet completely sure how will I use and update sequence so I left for later if some things need more clarification and improvements.

### 2025-01-09: Use BinaryHeap instead of VecDeque for FIFO queues

Changed from `VecDeque<Bid>` to `BinaryHeap<Bid>` for price-level queues. Initially, in my code I chose the right sequence number for bid(I use fetch_add atomic operation) but I'd update the queue after the bids lock is acquired, which meant that bids don't have to be added in the queue in the exact order they arrived. I needed something like a priority queue, when I add to the queue I wanted to sort it by the sequence. BinaryHeap does this.

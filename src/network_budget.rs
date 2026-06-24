use std::time::Instant;

/// Shared token bucket bounding how many AMX native calls (each one a
/// reliable RPC broadcast to every connected player) the plugin emits per
/// second, across every active screen combined. Without this, N screens
/// each spending their own fixed per-tick allowance multiplies the emitted
/// packet rate linearly with screen/tile count, which is what was tripping
/// the server's per-client ack rate limit.
pub struct NetworkBudget {
    tokens: f64,
    capacity: f64,
    rate_per_sec: f64,
    last_refill: Instant,
}

impl NetworkBudget {
    pub fn new(rate_per_sec: f64, capacity: f64) -> Self {
        NetworkBudget {
            tokens: capacity,
            capacity,
            rate_per_sec,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        self.last_refill = now;
    }

    /// How many `unit_cost`-sized operations the current balance covers,
    /// capped at `max`. Spends nothing - call `spend` with the amount
    /// actually used, since callers often request more than they end up
    /// having pending work for.
    pub fn affordable(&mut self, unit_cost: f64, max: usize) -> usize {
        self.refill();
        let affordable = (self.tokens / unit_cost).floor().max(0.0) as usize;
        affordable.min(max)
    }

    /// Debits a cost already incurred (e.g. computed from the real payload
    /// size of the work just performed, rather than a flat per-call guess).
    pub fn spend(&mut self, cost: f64) {
        self.tokens -= cost;
    }

    /// All-or-nothing spend for operations that can't be partially done
    /// (e.g. a buffer swap, which must reposition every tile atomically).
    pub fn try_spend(&mut self, cost: f64) -> bool {
        self.refill();
        if self.tokens >= cost {
            self.tokens -= cost;
            true
        } else {
            false
        }
    }
}

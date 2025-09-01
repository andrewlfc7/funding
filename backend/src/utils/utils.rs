pub fn is_usd_stable<S: AsRef<str>>(q: S) -> bool {
    matches!(q.as_ref(), "USDT" | "USDC")
}

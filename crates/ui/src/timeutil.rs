// SPDX-License-Identifier: MIT

/// Cross-platform delay used by search debounce and autosave.
pub async fn delay_ms(ms: u32) {
    #[cfg(feature = "web")]
    {
        gloo_timers::future::TimeoutFuture::new(ms).await;
    }
    #[cfg(feature = "desktop")]
    {
        tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    }
    #[cfg(not(any(feature = "web", feature = "desktop")))]
    {
        let _ = ms;
    }
}

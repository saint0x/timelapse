//! Stop the Timelapse daemon

use anyhow::Result;

pub async fn run() -> Result<()> {
    crate::daemon::stop().await
}

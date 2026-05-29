use std::time::Instant;

mod support;

use support::{LocalSyncHarness, env_usize, write_numbered_files};

#[tokio::test]
#[ignore = "performance benchmark; run explicitly with --ignored --nocapture"]
async fn local_fs_sync_benchmark_reports_throughput() -> Result<(), Box<dyn std::error::Error>> {
    let file_count = env_usize("HOARDER_BENCH_FILES", 500);
    let bytes_per_file = env_usize("HOARDER_BENCH_BYTES", 1024);
    let harness = LocalSyncHarness::new("performance-benchmark").await?;
    write_numbered_files(&harness.source_root, file_count, bytes_per_file)?;

    let engine = harness.sync_engine();
    let started = Instant::now();
    let summary = engine.run_job(harness.job_id).await?;
    let elapsed = started.elapsed();
    let elapsed_millis = elapsed.as_millis().max(1);
    let files_per_second = u128::from(summary.processed) * 1000 / elapsed_millis;
    let bytes_per_second = u128::from(summary.bytes_written) * 1000 / elapsed_millis;

    assert_eq!(summary.failed, 0);
    assert!(summary.processed >= u64::try_from(file_count)?);
    assert!(summary.synced >= u64::try_from(file_count)?);
    assert!(summary.bytes_written >= u64::try_from(file_count * bytes_per_file)?);

    println!(
        "benchmark files={} bytes_per_file={} processed={} synced={} bytes_written={} elapsed_ms={} files_per_second={} bytes_per_second={}",
        file_count,
        bytes_per_file,
        summary.processed,
        summary.synced,
        summary.bytes_written,
        elapsed_millis,
        files_per_second,
        bytes_per_second,
    );

    Ok(())
}

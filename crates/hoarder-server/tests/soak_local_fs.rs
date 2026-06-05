mod support;

use support::{LocalSyncHarness, env_usize, write_numbered_files};

#[tokio::test]
#[ignore = "long-running soak test; run explicitly with --ignored --nocapture"]
async fn local_fs_sync_soak_repeated_runs() -> Result<(), Box<dyn std::error::Error>> {
    let iterations = env_usize("HOARDER_SOAK_ITERATIONS", 30);
    let file_count = env_usize("HOARDER_SOAK_FILES", 100);
    let harness = LocalSyncHarness::new("soak-local-fs").await?;
    write_numbered_files(&harness.source_root, file_count, 256)?;

    let engine = harness.sync_engine();
    for iteration in 0..iterations {
        mutate_source_tree(&harness.source_root, iteration)?;
        let summary = engine.run_job(harness.job_id).await?;

        assert_eq!(summary.failed, 0, "iteration {iteration} failed");
        assert!(
            summary.processed > 0,
            "iteration {iteration} processed no items"
        );

        println!(
            "soak iteration={} processed={} synced={} skipped={} deleted={} bytes_written={}",
            iteration,
            summary.processed,
            summary.synced,
            summary.skipped,
            summary.deleted,
            summary.bytes_written,
        );
    }

    Ok(())
}

fn mutate_source_tree(
    source_root: &std::path::Path,
    iteration: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let churn_dir = source_root.join("churn");
    std::fs::create_dir_all(&churn_dir)?;

    std::fs::write(
        churn_dir.join("always-changing.txt"),
        format!("iteration={iteration}\n"),
    )?;

    if iteration % 3 == 0 {
        std::fs::write(
            churn_dir.join(format!("added-{iteration:04}.txt")),
            format!("added at iteration {iteration}\n"),
        )?;
    }

    if iteration > 0 && iteration % 5 == 0 {
        let deleted_candidate = churn_dir.join(format!("added-{:04}.txt", iteration - 3));
        if deleted_candidate.exists() {
            std::fs::remove_file(deleted_candidate)?;
        }
    }

    Ok(())
}

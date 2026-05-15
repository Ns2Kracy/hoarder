use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use hoarder::{
    config::{AppConfig, RuntimeSettingsPatch},
    db::{
        connect_sqlite,
        repository::{RuntimeSettingsRepository, SeaOrmRepository},
        schema::sync_schema,
    },
};

#[tokio::test]
async fn settings_repository_loads_config_defaults_from_empty_db()
-> Result<(), Box<dyn std::error::Error>> {
    let db = connect_sqlite("sqlite::memory:").await?;
    sync_schema(&db).await?;
    let repository = SeaOrmRepository::new(db);
    let config = test_config();

    let settings = repository.load_runtime_settings(&config).await?;

    assert_eq!(
        settings.database_path,
        config.database_path.to_string_lossy()
    );
    assert_eq!(settings.vault_path, config.vault_path.to_string_lossy());
    assert_eq!(settings.listen_addr, config.listen_addr);
    assert_eq!(settings.job_concurrency, config.job_concurrency);
    assert_eq!(settings.file_concurrency, config.file_concurrency);
    assert_eq!(settings.log_level, config.log_level);
    assert!(settings.read_only.database_path);
    assert!(settings.read_only.vault_path);
    assert!(settings.read_only.listen_addr);

    Ok(())
}

#[tokio::test]
async fn settings_repository_patches_and_reloads_mutable_runtime_settings()
-> Result<(), Box<dyn std::error::Error>> {
    let db = connect_sqlite("sqlite::memory:").await?;
    sync_schema(&db).await?;
    let repository = SeaOrmRepository::new(db);
    let config = test_config();

    let updated = repository
        .patch_runtime_settings(
            &config,
            RuntimeSettingsPatch {
                job_concurrency: Some(3),
                file_concurrency: Some(9),
                log_level: Some("debug".to_owned()),
            },
        )
        .await?;

    assert_eq!(
        updated.database_path,
        config.database_path.to_string_lossy()
    );
    assert_eq!(updated.vault_path, config.vault_path.to_string_lossy());
    assert_eq!(updated.listen_addr, config.listen_addr);
    assert_eq!(updated.job_concurrency, 3);
    assert_eq!(updated.file_concurrency, 9);
    assert_eq!(updated.log_level, "debug");

    let reloaded = repository.load_runtime_settings(&config).await?;
    assert_eq!(reloaded.job_concurrency, 3);
    assert_eq!(reloaded.file_concurrency, 9);
    assert_eq!(reloaded.log_level, "debug");
    assert_eq!(
        reloaded.database_path,
        config.database_path.to_string_lossy()
    );
    assert_eq!(reloaded.vault_path, config.vault_path.to_string_lossy());
    assert_eq!(reloaded.listen_addr, config.listen_addr);

    Ok(())
}

#[tokio::test]
async fn settings_repository_rejects_zero_concurrency_values()
-> Result<(), Box<dyn std::error::Error>> {
    let db = connect_sqlite("sqlite::memory:").await?;
    sync_schema(&db).await?;
    let repository = SeaOrmRepository::new(db);
    let config = test_config();

    let job_error = repository
        .patch_runtime_settings(
            &config,
            RuntimeSettingsPatch {
                job_concurrency: Some(0),
                file_concurrency: None,
                log_level: None,
            },
        )
        .await
        .expect_err("zero job concurrency should be rejected");
    assert!(
        job_error
            .to_string()
            .contains("job_concurrency must be greater than zero"),
        "unexpected error: {job_error}"
    );

    let file_error = repository
        .patch_runtime_settings(
            &config,
            RuntimeSettingsPatch {
                job_concurrency: None,
                file_concurrency: Some(0),
                log_level: None,
            },
        )
        .await
        .expect_err("zero file concurrency should be rejected");
    assert!(
        file_error
            .to_string()
            .contains("file_concurrency must be greater than zero"),
        "unexpected error: {file_error}"
    );

    Ok(())
}

fn test_config() -> AppConfig {
    AppConfig {
        database_path: PathBuf::from("/tmp/hoarder-settings.sqlite"),
        vault_path: PathBuf::from("/tmp/hoarder-vault"),
        listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4761),
        job_concurrency: 2,
        file_concurrency: 5,
        log_level: "warn".to_owned(),
    }
}

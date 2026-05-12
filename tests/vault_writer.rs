use bytes::Bytes;
use camino::Utf8PathBuf;
use futures::stream;
use hoarder::{
    connectors::traits::ByteStream,
    core::types::{ItemRef, ItemType, SourceId},
    sync::vault_writer::VaultWriter,
};

#[tokio::test]
async fn vault_writer_writes_stream_to_normalized_target_and_returns_hash() {
    let vault_root = temp_vault_root();
    let source_id = SourceId::new();
    let writer = VaultWriter::new(vault_root.clone());
    let item_ref = ItemRef {
        source_id,
        source_path: "docs/readme.md".to_owned(),
        item_type: ItemType::File,
    };
    let bytes = byte_stream([
        Ok(Bytes::from_static(b"hel")),
        Ok(Bytes::from_static(b"lo")),
    ]);

    let outcome = writer.write(&item_ref, bytes).await.unwrap();

    let target_path = vault_root
        .join(source_id.to_string())
        .join("docs/readme.md");
    assert_eq!(tokio::fs::read(&target_path).await.unwrap(), b"hello");
    assert_eq!(outcome.target_path, target_path);
    assert_eq!(
        outcome.content_hash,
        "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
    assert_eq!(outcome.bytes_written, 5);
}

#[tokio::test]
async fn vault_writer_removes_temp_file_and_never_promotes_partial_final_file_on_failure() {
    let vault_root = temp_vault_root();
    let source_id = SourceId::new();
    let writer = VaultWriter::new(vault_root.clone());
    let item_ref = ItemRef {
        source_id,
        source_path: "docs/readme.md".to_owned(),
        item_type: ItemType::File,
    };
    let bytes = byte_stream([
        Ok(Bytes::from_static(b"partial")),
        Err(std::io::Error::other("read failed").into()),
    ]);

    let error = writer.write(&item_ref, bytes).await.unwrap_err();

    assert!(error.to_string().contains("read failed"));
    assert!(
        tokio::fs::metadata(
            vault_root
                .join(source_id.to_string())
                .join("docs/readme.md")
        )
        .await
        .is_err()
    );
    let tmp_dir = vault_root.join(".hoarder/tmp");
    let mut entries = tokio::fs::read_dir(tmp_dir).await.unwrap();
    assert!(entries.next_entry().await.unwrap().is_none());
}

#[tokio::test]
async fn vault_writer_rejects_traversal_before_creating_final_path() {
    let vault_root = temp_vault_root();
    let source_id = SourceId::new();
    let writer = VaultWriter::new(vault_root.clone());
    let item_ref = ItemRef {
        source_id,
        source_path: "../escape.txt".to_owned(),
        item_type: ItemType::File,
    };

    let error = writer
        .write(&item_ref, byte_stream([Ok(Bytes::from_static(b"nope"))]))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("traversal"));
    assert!(
        tokio::fs::metadata(vault_root.join("escape.txt"))
            .await
            .is_err()
    );
}

fn byte_stream<const N: usize>(items: [Result<Bytes, hoarder::AppError>; N]) -> ByteStream {
    Box::pin(stream::iter(items))
}

fn temp_vault_root() -> Utf8PathBuf {
    let root = std::env::temp_dir().join(format!("hoarder-vault-writer-{}", SourceId::new()));
    let root = Utf8PathBuf::from_path_buf(root).unwrap();
    std::fs::remove_dir_all(&root).ok();
    root
}

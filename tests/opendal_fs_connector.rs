use std::{collections::BTreeMap, fs, path::PathBuf};

use bytes::Bytes;
use futures::StreamExt;
use hoarder::{
    connectors::{
        opendal::source::OpenDalSourceConnector,
        traits::{ConnectorConfig, SourceConnector},
    },
    core::types::{ItemType, SourceId},
};
use uuid::Uuid;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("hoarder-{name}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();

        Self { path }
    }

    fn path_string(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[tokio::test]
async fn opendal_fs_connector_lists_nested_files() {
    let root = TempDir::new("scan");
    fs::create_dir_all(root.path.join("docs/nested")).unwrap();
    fs::write(root.path.join("docs/readme.md"), "hello").unwrap();
    fs::write(root.path.join("docs/nested/guide.txt"), "nested").unwrap();

    let source_id = source_id();
    let connector = OpenDalSourceConnector::new(source_id);
    let config = fs_config(&root);

    let mut scan = connector.scan(&config, None).await.unwrap();
    let mut files = Vec::new();
    while let Some(snapshot) = scan.next().await {
        let snapshot = snapshot.unwrap();
        if snapshot.item_type == ItemType::File {
            files.push((snapshot.source_path, snapshot.source_id, snapshot.size));
        }
    }

    files.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        files,
        vec![
            ("docs/nested/guide.txt".to_owned(), source_id, Some(6)),
            ("docs/readme.md".to_owned(), source_id, Some(5)),
        ]
    );
}

#[tokio::test]
async fn opendal_fs_connector_reads_file_contents_as_stream() {
    let root = TempDir::new("read");
    fs::create_dir_all(root.path.join("docs")).unwrap();
    fs::write(root.path.join("docs/readme.md"), "hello from fs").unwrap();

    let connector = OpenDalSourceConnector::new(source_id());
    let config = fs_config(&root);

    let snapshot = {
        let mut scan = connector.scan(&config, None).await.unwrap();
        let mut found = None;
        while let Some(snapshot) = scan.next().await {
            let snapshot = snapshot.unwrap();
            if snapshot.source_path == "docs/readme.md" {
                found = Some(snapshot);
                break;
            }
        }
        found.expect("readme snapshot should be listed")
    };

    let mut stream = connector.read(&config, &snapshot.item_ref()).await.unwrap();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk: Bytes = chunk.unwrap();
        body.extend_from_slice(&chunk);
    }

    assert_eq!(body, b"hello from fs");
}

#[tokio::test]
async fn opendal_fs_connector_streams_file_without_single_buffering() {
    let root = TempDir::new("read-streaming");
    fs::create_dir_all(root.path.join("docs")).unwrap();
    fs::write(root.path.join("docs/large.bin"), vec![b'x'; 256 * 1024]).unwrap();

    let connector = OpenDalSourceConnector::new(source_id());
    let config = fs_config(&root);
    let snapshot = {
        let mut scan = connector.scan(&config, None).await.unwrap();
        let mut found = None;
        while let Some(snapshot) = scan.next().await {
            let snapshot = snapshot.unwrap();
            if snapshot.source_path == "docs/large.bin" {
                found = Some(snapshot);
                break;
            }
        }
        found.expect("large file snapshot should be listed")
    };

    let mut stream = connector.read(&config, &snapshot.item_ref()).await.unwrap();
    let mut chunk_count = 0;
    let mut bytes_read = 0;
    while let Some(chunk) = stream.next().await {
        let chunk: Bytes = chunk.unwrap();
        chunk_count += 1;
        bytes_read += chunk.len();
    }

    assert_eq!(bytes_read, 256 * 1024);
    assert!(
        chunk_count > 1,
        "streaming reads should yield multiple chunks for large files"
    );
}

fn fs_config(root: &TempDir) -> ConnectorConfig {
    ConnectorConfig::OpenDal {
        service: "fs".to_owned(),
        options: BTreeMap::from([("root".to_owned(), root.path_string())]),
    }
}

fn source_id() -> SourceId {
    SourceId::from_uuid(Uuid::parse_str("018f3f55-6b4d-7b2f-8b1e-f7563f31b8d5").unwrap())
}

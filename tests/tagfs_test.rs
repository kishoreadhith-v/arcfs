use better_fs::file_manager::FileManager;
use std::fs;
use std::path::Path;

fn setup(path: &str) -> FileManager {
    if Path::new(path).exists() {
        fs::remove_dir_all(path).expect("cleanup failed");
    }
    FileManager::new(path)
}

#[test]
fn tagfs_set_and_get_tags() {
    let path = "./test_tagfs_1";
    let manager = setup(path);

    manager
        .set_file_tags(101, "doc.txt", vec!["Work".into(), "2026".into(), "work".into()])
        .expect("set tags failed");

    let tags = manager.get_file_tags(101).expect("get tags failed");
    assert_eq!(tags, vec!["2026".to_string(), "work".to_string()]);

    fs::remove_dir_all(path).expect("cleanup failed");
}

#[test]
fn tagfs_query_single_tag() {
    let path = "./test_tagfs_2";
    let manager = setup(path);

    manager
        .set_file_tags(201, "a.txt", vec!["work".into(), "backend".into()])
        .expect("set tags failed");
    manager
        .set_file_tags(202, "b.txt", vec!["work".into(), "frontend".into()])
        .expect("set tags failed");

    let files = manager.get_files_with_tag("work").expect("query failed");
    assert_eq!(files, vec![201, 202]);

    fs::remove_dir_all(path).expect("cleanup failed");
}

#[test]
fn tagfs_query_multiple_tags_intersection() {
    let path = "./test_tagfs_3";
    let manager = setup(path);

    manager
        .set_file_tags(301, "a.txt", vec!["work".into(), "2026".into(), "backend".into()])
        .expect("set tags failed");
    manager
        .set_file_tags(302, "b.txt", vec!["work".into(), "2026".into(), "frontend".into()])
        .expect("set tags failed");
    manager
        .set_file_tags(303, "c.txt", vec!["work".into(), "2025".into()])
        .expect("set tags failed");

    let files = manager
        .get_files_by_tags(&["work".into(), "2026".into()])
        .expect("intersection failed");
    assert_eq!(files, vec![301, 302]);

    let backend = manager
        .get_files_by_tags(&["work".into(), "2026".into(), "backend".into()])
        .expect("intersection failed");
    assert_eq!(backend, vec![301]);

    fs::remove_dir_all(path).expect("cleanup failed");
}

#[test]
fn tagfs_next_level_tags() {
    let path = "./test_tagfs_4";
    let manager = setup(path);

    manager
        .set_file_tags(401, "a.txt", vec!["projects".into(), "work".into(), "2026".into()])
        .expect("set tags failed");
    manager
        .set_file_tags(402, "b.txt", vec!["projects".into(), "personal".into(), "2026".into()])
        .expect("set tags failed");

    let next = manager
        .get_next_level_tags(&["projects".into()])
        .expect("next-level tags failed");

    assert_eq!(
        next,
        vec!["2026".to_string(), "personal".to_string(), "work".to_string()]
    );

    fs::remove_dir_all(path).expect("cleanup failed");
}

#[test]
fn tagfs_delete_file_tags_updates_index() {
    let path = "./test_tagfs_5";
    let manager = setup(path);

    manager
        .set_file_tags(501, "x.txt", vec!["work".into(), "docs".into()])
        .expect("set tags failed");

    manager.delete_file_tags(501).expect("delete tags failed");

    let tags = manager.get_file_tags(501).expect("get tags failed");
    assert!(tags.is_empty());

    let files = manager.get_files_with_tag("work").expect("query failed");
    assert!(files.is_empty());

    fs::remove_dir_all(path).expect("cleanup failed");
}

#[test]
fn tagfs_persistence_across_restart() {
    let path = "./test_tagfs_6";
    {
        let manager = setup(path);
        manager
            .set_file_tags(601, "persist.txt", vec!["work".into(), "2026".into()])
            .expect("set tags failed");
    }

    let manager = FileManager::new(path);
    let tags = manager.get_file_tags(601).expect("get tags failed");
    assert_eq!(tags, vec!["2026".to_string(), "work".to_string()]);

    let files = manager
        .get_files_by_tags(&["work".into(), "2026".into()])
        .expect("query failed");
    assert_eq!(files, vec![601]);

    fs::remove_dir_all(path).expect("cleanup failed");
}

// tests/tagfs_test.rs
//
// Comprehensive test suite for Phase 3: TagFS (Semantic Tagging)
// Tests order-independent file access via tag permutations

#[path = "../src/chunker.rs"]
mod chunker;
#[path = "../src/file_manager.rs"]
mod file_manager;
#[path = "../src/storage.rs"]
mod storage;

use file_manager::FileManager;
use std::fs;
use std::path::Path;

/// Setup test environment with fresh database
fn setup_tagfs(base_path: &str) -> FileManager {
    if Path::new(base_path).exists() {
        fs::remove_dir_all(base_path).unwrap();
    }
    FileManager::new(base_path)
}

/// Test 1: Basic tag storage and retrieval
#[test]
fn test_tagfs_1_basic_tag_storage() {
    let path = "./test_tagfs_1";
    let manager = setup_tagfs(path);

    let inode_id = 101u64;
    let filename = "document.pdf";
    let tags = vec!["work".to_string(), "2026".to_string(), "projects".to_string()];

    // Store tags
    assert!(manager.set_file_tags(inode_id, filename, tags.clone()).is_ok());

    // Retrieve tags
    let retrieved_tags = manager.get_file_tags(inode_id).expect("Failed to retrieve tags");
    assert_eq!(retrieved_tags, tags);
    println!("✓ Basic tag storage test passed");
}

/// Test 2: Query files with single tag
#[test]
fn test_tagfs_2_query_single_tag() {
    let path = "./test_tagfs_2";
    let manager = setup_tagfs(path);

    // Create multiple files with overlapping tags
    let files = vec![
        (101u64, "file1.txt", vec!["work", "2026"]),
        (102u64, "file2.txt", vec!["work", "2025"]),
        (103u64, "file3.txt", vec!["personal", "2026"]),
    ];

    for (inode_id, filename, tags) in files {
        let tag_vec: Vec<String> = tags.iter().map(|&s| s.to_string()).collect();
        manager.set_file_tags(inode_id, filename, tag_vec).unwrap();
    }

    // Query: files with tag "work"
    let work_files = manager.get_files_with_tag("work").expect("Query failed");
    assert_eq!(work_files.len(), 2);
    assert!(work_files.contains(&101));
    assert!(work_files.contains(&102));
    println!("✓ Single tag query test passed");
}

/// Test 3: Query files with multiple tags (intersection)
#[test]
fn test_tagfs_3_query_multiple_tags_intersection() {
    let path = "./test_tagfs_3";
    let manager = setup_tagfs(path);

    // Files with different tag combinations
    let files = vec![
        (101u64, "file1.txt", vec!["work", "2026", "backend"]),
        (102u64, "file2.txt", vec!["work", "2026", "frontend"]),
        (103u64, "file3.txt", vec!["work", "2025", "backend"]),
        (104u64, "file4.txt", vec!["personal", "2026", "backend"]),
    ];

    for (inode_id, filename, tags) in files {
        let tag_vec: Vec<String> = tags.iter().map(|&s| s.to_string()).collect();
        manager.set_file_tags(inode_id, filename, tag_vec).unwrap();
    }

    // Query: files with BOTH "work" AND "2026"
    let query_tags = vec!["work".to_string(), "2026".to_string()];
    let results = manager.get_files_by_tags(&query_tags).expect("Query failed");
    assert_eq!(results.len(), 2);
    assert!(results.contains(&101)); // work + 2026 + backend
    assert!(results.contains(&102)); // work + 2026 + frontend
    assert!(!results.contains(&103)); // work + 2025 (missing 2026)
    assert!(!results.contains(&104)); // personal (missing work)

    println!("✓ Multiple tags intersection query test passed");
}

/// Test 4: Order-independent access simulation
/// (In real mounting, all permutations would work transparently)
#[test]
fn test_tagfs_4_tag_permutation_simulation() {
    let path = "./test_tagfs_4";
    let manager = setup_tagfs(path);

    let inode_id = 101u64;
    let filename = "report.xlsx";
    let tags = vec!["projects".to_string(), "backend".to_string(), "2026".to_string()];

    manager.set_file_tags(inode_id, filename, tags.clone()).unwrap();

    // All these permutations should query the same file:
    let permutations = vec![
        vec!["projects", "backend", "2026"],     // Original order
        vec!["projects", "2026", "backend"],     // Permutation 1
        vec!["backend", "projects", "2026"],     // Permutation 2
        vec!["backend", "2026", "projects"],     // Permutation 3
        vec!["2026", "projects", "backend"],     // Permutation 4
        vec!["2026", "backend", "projects"],     // Permutation 5
    ];

    for perm in permutations {
        let query_tags: Vec<String> = perm.iter().map(|&s| s.to_string()).collect();
        let results = manager.get_files_by_tags(&query_tags).expect("Query failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], inode_id);
        println!("  ✓ Query {:?} found inode {}", query_tags, inode_id);
    }

    println!("✓ Tag permutation simulation test passed");
}

/// Test 5: Next-level tags discovery
/// Given current tags, find possible next tags to navigate deeper
#[test]
fn test_tagfs_5_next_level_tags() {
    let path = "./test_tagfs_5";
    let manager = setup_tagfs(path);

    // Files in a hierarchical tag structure
    let files = vec![
        (101u64, "api.rs", vec!["projects", "backend", "2026"]),
        (102u64, "main.rs", vec!["projects", "backend", "2026"]),
        (103u64, "index.js", vec!["projects", "frontend", "2026"]),
        (104u64, "config.yml", vec!["projects", "devops", "2026"]),
        (105u64, "specs.md", vec!["docs", "2026"]),
    ];

    for (inode_id, filename, tags) in files {
        let tag_vec: Vec<String> = tags.iter().map(|&s| s.to_string()).collect();
        manager.set_file_tags(inode_id, filename, tag_vec).unwrap();
    }

    // Test: Given "projects", what are the next possible tags?
    let next = manager.get_next_level_tags(&["projects".to_string()]).expect("Query failed");
    assert!(next.contains(&"backend".to_string()));
    assert!(next.contains(&"frontend".to_string()));
    assert!(next.contains(&"devops".to_string()));
    assert!(!next.contains(&"docs".to_string())); // docs files don't have "projects"
    println!("  ✓ Next tags from 'projects': {:?}", next);

    // Test: Given "projects" and "backend", what are the next possible tags?
    let next = manager.get_next_level_tags(&["projects".to_string(), "backend".to_string()])
        .expect("Query failed");
    assert!(next.contains(&"2026".to_string()));
    assert!(!next.contains(&"frontend".to_string())); // frontend files don't have "backend" too
    println!("  ✓ Next tags from ['projects', 'backend']: {:?}", next);

    println!("✓ Next-level tags discovery test passed");
}

/// Test 6: Complex tag filtering with partial matches
#[test]
fn test_tagfs_6_partial_tag_matching() {
    let path = "./test_tagfs_6";
    let manager = setup_tagfs(path);

    // Create files with various tag combinations (using unique inode IDs to avoid collisions)
    let files = vec![
        (6001u64, "article1.md", vec!["blog", "rust", "2026"]),
        (6002u64, "article2.md", vec!["blog", "python", "2026"]),
        (6003u64, "article3.md", vec!["blog", "rust", "2025"]),
        (6004u64, "code.rs", vec!["rust", "implementation"]),
    ];

    for (inode_id, filename, tags) in files {
        let tag_vec: Vec<String> = tags.iter().map(|&s| s.to_string()).collect();
        manager.set_file_tags(inode_id, filename, tag_vec).unwrap();
    }

    // Query: blog + rust
    let results = manager.get_files_by_tags(&[
        "blog".to_string(),
        "rust".to_string(),
    ]).expect("Query failed");
    assert_eq!(results.len(), 2, "Expected 2 files with blog+rust, got {}: {:?}", results.len(), results);
    assert!(results.contains(&6001)); // Only article1.md and article3.md have both blog AND rust
    assert!(results.contains(&6003));
    println!("  ✓ Blog + Rust: found inodes {:?}", results);

    // Query: blog + rust + 2026
    let results = manager.get_files_by_tags(&[
        "blog".to_string(),
        "rust".to_string(),
        "2026".to_string(),
    ]).expect("Query failed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 6001); // Must have all three tags
    println!("  ✓ Blog + Rust + 2026: found inode {}", results[0]);

    // Query: rust (should find 3 files)
    let results = manager.get_files_with_tag("rust").expect("Query failed");
    assert_eq!(results.len(), 3);
    println!("  ✓ Rust files: found {} inodes", results.len());

    println!("✓ Partial tag matching test passed");
}

/// Test 7: Tag persistence across manager restarts
#[test]
fn test_tagfs_7_persistence() {
    let path = "./test_tagfs_7";

    // Session 1: Create and tag files
    {
        let manager = setup_tagfs(path);
        manager.set_file_tags(
            101u64,
            "important.doc",
            vec!["archive".to_string(), "2025".to_string()],
        ).unwrap();
    } // Manager dropped, database closed

    // Session 2: Reopen and verify tags survived
    let manager = FileManager::new(path);
    let tags = manager.get_file_tags(101).expect("Tags lost after restart!");
    assert_eq!(tags.len(), 2);
    assert!(tags.contains(&"archive".to_string()));
    assert!(tags.contains(&"2025".to_string()));
    println!("✓ Tag persistence test passed");
}

/// Test 8: Empty tag sets and edge cases
#[test]
fn test_tagfs_8_edge_cases() {
    let path = "./test_tagfs_8";
    let manager = setup_tagfs(path);

    // File with no tags
    manager.set_file_tags(101u64, "untagged.txt", vec![]).unwrap();
    let tags = manager.get_file_tags(101).expect("Failed to get empty tags");
    assert_eq!(tags.len(), 0);

    // Query with empty tag list (should return no results per current logic)
    let results = manager.get_files_by_tags(&[]).expect("Empty query failed");
    assert_eq!(results.len(), 0);

    // Query non-existent tag
    let results = manager.get_files_with_tag("nonexistent").expect("Query failed");
    assert_eq!(results.len(), 0);

    // Overwrite existing tags
    manager.set_file_tags(
        102u64,
        "file2.txt",
        vec!["old_tag".to_string()],
    ).unwrap();
    manager.set_file_tags(
        102u64,
        "file2.txt",
        vec!["new_tag".to_string(), "another".to_string()],
    ).unwrap();
    let tags = manager.get_file_tags(102).expect("Failed to get updated tags");
    assert_eq!(tags.len(), 2);
    assert!(tags.contains(&"new_tag".to_string()));
    assert!(tags.contains(&"another".to_string()));

    println!("✓ Edge cases test passed");
}

/// Test 9: Large-scale tag queries
#[test]
fn test_tagfs_9_large_scale() {
    let path = "./test_tagfs_9";
    let manager = setup_tagfs(path);

    // Create 100 files with various tag combinations
    let years = vec!["2024", "2025", "2026"];
    let categories = vec!["blog", "code", "docs", "tutorial"];
    let tags_list = vec!["rust", "python", "javascript"];

    let mut inode = 1000u64;
    let mut total_files = 0;

    for year in &years {
        for category in &categories {
            for tag in &tags_list {
                let mut file_tags = vec![
                    year.to_string(),
                    category.to_string(),
                    tag.to_string(),
                ];
                // Some files get an extra tag
                if inode % 3 == 0 {
                    file_tags.push("featured".to_string());
                }

                manager.set_file_tags(
                    inode,
                    &format!("file_{}.txt", inode),
                    file_tags,
                ).unwrap();
                inode += 1;
                total_files += 1;
            }
        }
    }

    // Query: All "rust" files
    let rust_files = manager.get_files_with_tag("rust").expect("Query failed");
    assert_eq!(rust_files.len(), 12); // 1 tag × 4 categories × 3 years

    // Query: "rust" AND "2026"
    let results = manager.get_files_by_tags(&[
        "rust".to_string(),
        "2026".to_string(),
    ]).expect("Query failed");
    assert_eq!(results.len(), 4); // rust × 4 categories × 1 year

    // Query: "rust" AND "blog" AND "2026"
    let results = manager.get_files_by_tags(&[
        "rust".to_string(),
        "blog".to_string(),
        "2026".to_string(),
    ]).expect("Query failed");
    assert_eq!(results.len(), 1); // Only one file matches all three

    // Query: "featured" tag (subset of files)
    let featured = manager.get_files_with_tag("featured").expect("Query failed");
    assert!(featured.len() > 0);
    assert!(featured.len() < total_files);
    println!("  ✓ Featured files: {} out of {}", featured.len(), total_files);

    println!("✓ Large-scale tag queries test passed");
}

/// Test 10: Tag isolation between multiple files (no leakage)
#[test]
fn test_tagfs_10_tag_isolation() {
    let path = "./test_tagfs_10";
    let manager = setup_tagfs(path);

    // Create two files with completely different tags
    let file1_tags = vec!["alpha".to_string(), "beta".to_string()];
    let file2_tags = vec!["gamma".to_string(), "delta".to_string()];

    manager.set_file_tags(101u64, "file1.txt", file1_tags).unwrap();
    manager.set_file_tags(102u64, "file2.txt", file2_tags).unwrap();

    // Verify no tag leakage
    let f1_tags = manager.get_file_tags(101).expect("Failed to get file1 tags");
    let f2_tags = manager.get_file_tags(102).expect("Failed to get file2 tags");

    assert_eq!(f1_tags.len(), 2);
    assert!(f1_tags.contains(&"alpha".to_string()));
    assert!(f1_tags.contains(&"beta".to_string()));
    assert!(!f1_tags.contains(&"gamma".to_string()));

    assert_eq!(f2_tags.len(), 2);
    assert!(f2_tags.contains(&"gamma".to_string()));
    assert!(f2_tags.contains(&"delta".to_string()));
    assert!(!f2_tags.contains(&"alpha".to_string()));

    // Query "alpha" should only return file1
    let alpha_results = manager.get_files_with_tag("alpha").expect("Query failed");
    assert_eq!(alpha_results.len(), 1);
    assert_eq!(alpha_results[0], 101);

    println!("✓ Tag isolation test passed");
}

/// Test 11: Real-world scenario - project file organization
#[test]
fn test_tagfs_11_project_organization() {
    let path = "./test_tagfs_11_isolated";
    let manager = setup_tagfs(path);

    // Simulate a project structure with automatic tagging (using very high inode IDs for isolation)
    let project_files = vec![
        (70001u64, "config.yml", vec!["deploy", "devops", "2026"]),
        (70002u64, "README.md", vec!["docs", "2026"]),
        (70003u64, "schema.sql", vec!["database", "2026"]),
        (70004u64, "tests.py", vec!["testing", "python", "2026"]),
        (70005u64, "requirements.txt", vec!["dependencies", "python", "2026"]),
        (70006u64, "Dockerfile", vec!["containers", "docker", "2026"]),
    ];

    for (inode_id, filename, tags) in project_files {
        let tag_vec: Vec<String> = tags.iter().map(|&s| s.to_string()).collect();
        manager.set_file_tags(inode_id, filename, tag_vec).unwrap();
    }

    // Query: All 2026 files (should be everything)
    let year_2026 = manager.get_files_with_tag("2026").expect("Query failed");
    assert_eq!(year_2026.len(), 6, "Expected 6 files tagged with 2026, got {}", year_2026.len());

    // Query: All deployment-related files
    let deploy = manager.get_files_by_tags(&[
        "deploy".to_string(),
        "devops".to_string(),
    ]).expect("Query failed");
    assert_eq!(deploy.len(), 1, "Expected 1 deployment file");
    assert!(deploy.contains(&70001));

    // Query: All python-related files
    let python_files = manager.get_files_with_tag("python").expect("Query failed");
    assert_eq!(python_files.len(), 2, "Expected 2 Python files");
    assert!(python_files.contains(&70004));
    assert!(python_files.contains(&70005));

    // Query: All docker-related files
    let docker_files = manager.get_files_with_tag("docker").expect("Query failed");
    assert_eq!(docker_files.len(), 1, "Expected 1 Docker file");
    assert_eq!(docker_files[0], 70006);

    // Query: Files with dependencies tag
    let deps = manager.get_files_with_tag("dependencies").expect("Query failed");
    assert_eq!(deps.len(), 1, "Expected 1 dependencies file");
    assert_eq!(deps[0], 70005);

    println!("✓ Project organization test passed");
}

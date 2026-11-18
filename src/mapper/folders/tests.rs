use super::folder_list::sort_library_dirs;
use crate::entity::library::library_dirs::LibraryDir;

fn dir(name: &str, files: i64, size: i64) -> LibraryDir {
    LibraryDir { name: name.into(), total_files: files, total_size: size, ..Default::default() }
}

#[test]
fn library_folders_follow_the_chosen_sort() {
    let dirs = [dir("Musique", 10, 300), dir("Archives", 50, 100), dir("Concerts", 20, 900)];
    let names = |sort| {
        let mut l: Vec<&LibraryDir> = dirs.iter().collect();
        sort_library_dirs(&mut l, sort);
        l.iter().map(|d| d.name.clone()).collect::<Vec<_>>()
    };
    assert_eq!(names("name"), ["Archives", "Concerts", "Musique"]);
    assert_eq!(names("files"), ["Archives", "Concerts", "Musique"]);
    assert_eq!(names("size"), ["Concerts", "Musique", "Archives"]);
}

use crate::vfs::{FileSystem, INodeNum, INodeType, OwnedDirEntry};
use std::io::prelude::*;

type StdPath = std::path::Path;
type StdPathBuf = std::path::PathBuf;

fn read_only_test_on<F: FileSystem>(fs: &mut F, fs_dir_inode: INodeNum, host_directory: &StdPath) {
    // compare directory entries
    let mut dir_handle = fs.open(fs_dir_inode).unwrap();
    let fs_dir_ents = fs.readdir(&mut dir_handle).unwrap().to_sorted_vec();
    let mut host_dir_ents: Vec<OwnedDirEntry> = std::fs::read_dir(host_directory)
        .unwrap()
        .filter_map(|std_dirent| {
            let std_dirent = std_dirent.unwrap();
            let name = std_dirent
                .file_name()
                .to_str()
                .expect("bad UTF-8 in host filename")
                .to_owned();
            if name == "." || name == ".." {
                return None;
            }
            let file_type = std_dirent.file_type().unwrap();
            let r#type = if file_type.is_symlink() {
                INodeType::Link
            } else if file_type.is_dir() {
                INodeType::Directory
            } else if file_type.is_file() {
                INodeType::File
            } else {
                panic!("Weird file type in host directory: {file_type:?}");
            };
            Some(OwnedDirEntry {
                inode: 0,
                name: name.into(),
                r#type,
            })
        })
        .collect();
    host_dir_ents.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(host_dir_ents.len(), fs_dir_ents.len());
    for (host_ent, fs_ent) in host_dir_ents.iter().zip(fs_dir_ents.iter()) {
        let mut host_subpath = StdPathBuf::from(host_directory);
        host_subpath.push(host_ent.name.as_ref());
        assert_eq!(host_ent.name, fs_ent.name);
        assert_eq!(host_ent.r#type, fs_ent.r#type);
        match fs_ent.r#type {
            INodeType::Directory => {
                read_only_test_on(fs, fs_ent.inode, &host_subpath);
            }
            INodeType::File => {
                let mut file = fs.open(fs_ent.inode).unwrap();
                // weird buffer size to try to catch edge cases (e.g. read crossing sector)
                let mut buffer = [0u8; 37];
                let mut fs_contents = vec![];
                loop {
                    let n = fs
                        .read(&mut file, fs_contents.len() as u64, &mut buffer)
                        .unwrap();
                    if n == 0 {
                        break;
                    }
                    fs_contents.extend_from_slice(&buffer[..n]);
                }
                let mut host_contents = vec![];
                std::fs::File::open(&host_subpath)
                    .unwrap()
                    .read_to_end(&mut host_contents)
                    .unwrap();
                assert_eq!(
                    host_contents,
                    fs_contents,
                    "mismatch at file {}",
                    host_subpath.to_string_lossy()
                );
            }
            INodeType::Link => todo!(),
        }
    }
}

/// ensure the filesystem matches the given directory on disk
pub fn read_only_test<F: FileSystem>(fs: &mut F, host_directory: impl AsRef<StdPath>) {
    let root = fs.root();
    read_only_test_on(fs, root, host_directory.as_ref());
}

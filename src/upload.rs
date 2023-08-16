use std::path::PathBuf;

// let mut handle = |m: Result<PrepManga>| match m {
//     Ok(manga) => {
//         let num_missing = manga.missing_chapters.len();
//         if num_missing > 0 {
//             warn!(
//                 "'{}' is missing {} chapter(s)",
//                 manga.info.name, num_missing
//             );
//             mangas.push_back(manga);
//         } else {
//             debug!("'{}' not missing any chapters", manga.info.name);
//         }
//     }
//     Err(Error::NoMangaSpec(path)) => {
//         error!("{:?} is missing 'manga.json'", path)
//     }
//     Err(Error::NoSeriesInfo(path)) => {
//         error!("{:?} is missing 'series.json'", path)
//     }
//     Err(Error::NoCoverImage(path)) => {
//         error!("{:?} is missing 'cover[.png|.jpg]'", path)
//     }
//     Err(Error::InvalidMangaSpec(path)) => {
//         error!("{:?} is a invalid 'manga.json'", path)
//     }
//     Err(Error::InvalidSeriesInfo(path)) => {
//         error!("{:?} is a invalid 'series.json'", path)
//     }
//     Err(e) => error!("Unknown error: {:?}", e),
// };

pub fn upload(_endpoint: String, _manga: Option<String>) {
}

// pub fn single(endpoint: String, path: PathBuf) {
//     // let manga = prep_manga(path, endpoint);
//     // handle(manga);
// }

// pub fn multiple(endpoint: String, dir: PathBuf) {
//     // let paths = dir.read_dir().unwrap();
//     // for path in paths {
//     //     let path = path.unwrap();
//     //     let path = path.path();
//     //
//     //     trace!("Looking at {:?}", path);
//     //     let manga = prep_manga(path, endpoint.clone());
//     //     handle(manga);
//     // }
// }

// if mangas.len() <= 0 {
//     println!("Nothing to upload (exiting)");
//     return;
// }
//
// println!("Num mangas to upload: {}", mangas.len());
// info!("-----------------");
// for manga in mangas.iter() {
//     info!(
//         "{} at {:?} needs to upload {} chapter(s)",
//         manga.info.name,
//         manga.paths.base,
//         manga.missing_chapters.len()
//     );
// }
// info!("-----------------");
//
// let total_missing_chapters = mangas
//     .iter()
//     .fold(0usize, |sum, val| sum + val.missing_chapters.len());
// debug!("Total missing chapters: {}", total_missing_chapters);
//
// let mut num_threads = args.num_threads;
// if total_missing_chapters < num_threads {
//     num_threads = total_missing_chapters;
// }
//
// info!("Using {} threads", num_threads);
//
// let mut missing_chapters = VecDeque::new();
// for (manga_index, manga) in mangas.iter().enumerate() {
//     for (chapter_index, _) in manga.missing_chapters.iter().enumerate() {
//         missing_chapters.push_back(MissingChapter {
//             manga_index,
//             chapter_index,
//         });
//     }
// }
//
// // println!("Chapters: {:#?}", missing_chapters);
//
// let mangas = Arc::new(RwLock::new(mangas));
// let work_queue = Arc::new(Mutex::new(missing_chapters));
//
// let mut thread_handles = Vec::new();
// for tid in 0..num_threads {
//     let work_queue_handle = work_queue.clone();
//     let mangas_handle = mangas.clone();
//     let handle = std::thread::spawn(move || {
//         worker_thread(tid, mangas_handle, work_queue_handle);
//     });
//     thread_handles.push(handle);
// }
//
// loop {
//     let left = {
//         let lock =
//             work_queue.lock().expect("Failed to get work queue lock");
//         lock.len()
//     };
//
//     let num_done = total_missing_chapters - left;
//     println!(
//         "Num Done: {}",
//         (num_done as f32 / total_missing_chapters as f32) * 100.0
//     );
//     std::thread::sleep(Duration::from_millis(750));
//
//     if left <= 0 {
//         break;
//     }
// }
//
// for handle in thread_handles {
//     handle.join().unwrap();
// }

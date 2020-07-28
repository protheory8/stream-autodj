use core::fmt::Formatter;
use core::fmt::Display;
use std::fs::File;
use std::io::BufReader;
use std::collections::HashSet;
use std::collections::HashMap;
use std::fs;
use rodio::Sink;
use rand::Rng;
use rand::seq::SliceRandom;

// Do NOT use mp3.

// Song:
// - id: string
// - has_end: bool
// - multi_loop_count: bool
// - valid_transitions: dict[loop num] = array of loop nums

struct Song {
	id: String,
	has_end: bool,
	multi_loop_count: i32,
	valid_transitions: HashMap<i32, Vec<i32>>
}

impl Display for Song {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} (has_end: {}, loop_count: {}, transitions: {})", self.id, self.has_end, self.multi_loop_count, self.valid_transitions.len())
	}
}

fn main() {
	let paths = fs::read_dir("./songs").unwrap();
	let mut unique_songs = HashSet::new();
	for path in paths.map(|p| p.unwrap().path().display().to_string()) {
		let file_name = path.split('/').rev().nth(0).unwrap().clone().to_string();
		let song_num = file_name.split('_').nth(1).unwrap();
		unique_songs.insert(song_num.to_owned());
	}
	let mut sorted_song_nums = unique_songs.into_iter().collect::<Vec<_>>();
	sorted_song_nums.sort();

	let mut songs = vec![];

	for song in sorted_song_nums {
		let has_end = fs::metadata(format!("songs/song_{}_end.ogg", song)).is_ok();
		let mut multi_loop_count = 0;
		if fs::metadata(format!("songs/song_{}_loop.ogg", song)).is_ok() {
			multi_loop_count = 1;
		}
		else {
			while fs::metadata(format!("songs/song_{}_loop{}.ogg", song, multi_loop_count)).is_ok() {
				multi_loop_count += 1;
			}
		}

		let mut valid_transitions = HashMap::new();

		for from in 0..multi_loop_count {
			for to in 0..multi_loop_count {
				if from == to {
					continue;
				}
				if fs::metadata(format!("songs/song_{}_loop{}-to-{}.ogg", song, from, to)).is_ok() {
					if !valid_transitions.contains_key(&from) {
						valid_transitions.insert(from, Vec::new());
					}
					valid_transitions.get_mut(&from).unwrap().push(to);
				}
			}
		}

		songs.push(Song {
			id: song,
			has_end,
			multi_loop_count,
			valid_transitions,
		});
	}

	// format!("song_{}_start.ogg", song),
	// format!("song_{}_loop.ogg", song),
	// format!("song_{}_end.ogg", song),

	let mut rng = rand::thread_rng();

	let device = rodio::default_output_device().unwrap();
	let sink = Sink::new(&device);

	loop {
		let song_num = rng.gen_range(0, songs.len());
		let current_song = &songs[song_num];

		let file_start = File::open(format!("songs/song_{}_start.ogg", current_song.id.as_str())).unwrap();
		let source_start = rodio::Decoder::new(BufReader::new(file_start)).unwrap();

		sink.append(source_start);
		if current_song.multi_loop_count == 1 {
			let repeat_count = rng.gen_range(3, 15);
			for _ in 0..repeat_count {
				let file_loop = File::open(format!("songs/song_{}_loop.ogg", current_song.id.as_str())).unwrap();
				let source_loop = rodio::Decoder::new(BufReader::new(file_loop)).unwrap();
				sink.append(source_loop);
			}
			println!("playing: song {}, repeated {} times", current_song, repeat_count);
		}
		else if current_song.valid_transitions.is_empty() {
			let loop_transitions = rng.gen_range(1, 4);
			let loop_plays = (0..loop_transitions).map(|_| {
				(rng.gen_range(0, current_song.multi_loop_count), rng.gen_range(3, 15))
			}).collect::<Vec<_>>();
			println!("playing: song {}, {} loop transitions, repeated {:?} times", current_song, loop_transitions, &loop_plays);
			for (loop_num, repeats) in loop_plays {
				for _ in 0..repeats {
					let file_loop = File::open(format!("songs/song_{}_loop{}.ogg", current_song.id.as_str(), loop_num)).unwrap();
					let source_loop = rodio::Decoder::new(BufReader::new(file_loop)).unwrap();
					sink.append(source_loop);
				}
			}
		}
		else {
			let loop_transitions = rng.gen_range(1, 4);
			println!("playing: song {}, {} loop transitions with special loop transitions", current_song, loop_transitions);
			let mut current_loop_num = 0;
			let mut flow = vec![];
			for _ in 0..loop_transitions {
				let possible_next_loops = &current_song.valid_transitions[&current_loop_num];
				current_loop_num = *possible_next_loops.choose(&mut rng).unwrap();
				flow.push(current_loop_num);
			}

			let repeats = rng.gen_range(3, 7);
			current_loop_num = 0;
			for _ in 0..repeats {
				let file_loop = File::open(format!("songs/song_{}_loop{}.ogg", current_song.id.as_str(), current_loop_num)).unwrap();
				let source_loop = rodio::Decoder::new(BufReader::new(file_loop)).unwrap();
				sink.append(source_loop);
			}

			for loop_num in flow {
				let file_transition = File::open(format!("songs/song_{}_loop{}-to-{}.ogg", current_song.id.as_str(), current_loop_num, loop_num)).unwrap();
				let source_transition = rodio::Decoder::new(BufReader::new(file_transition)).unwrap();
				sink.append(source_transition);
				current_loop_num = loop_num;

				let repeats = rng.gen_range(3, 7);
				for _ in 0..repeats {
					let file_loop = File::open(format!("songs/song_{}_loop{}.ogg", current_song.id.as_str(), loop_num)).unwrap();
					let source_loop = rodio::Decoder::new(BufReader::new(file_loop)).unwrap();
					sink.append(source_loop);
				}
			}
		}
		if current_song.has_end {
			let file_end = File::open(format!("songs/song_{}_end.ogg", current_song.id.as_str())).unwrap();
			let source_end = rodio::Decoder::new(BufReader::new(file_end)).unwrap();
			sink.append(source_end);
		}
		sink.sleep_until_end();
	}
}
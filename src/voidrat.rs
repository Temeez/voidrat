use crate::parsers::world_state::WorldState;
use crate::parsers::{CetusCycle, Fissure, TennoParser};

use bincode::{config, decode_from_std_read, encode_into_std_write};
use chrono::{DateTime, Duration, Local, Utc};
use log::{debug, warn};
use parking_lot::RwLock;

use std::env::current_dir;
use std::fs::{create_dir, File};
use std::io::BufWriter;

use std::path::PathBuf;

use crate::parsers::warframestat::WarframeStat;
use filetime::FileTime;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::{fs, thread};

const STORAGE_FILE: &str = "voidrat.storage";
const DATA_PATH: &str = "data";
const WORLD_STATE_DATA_PATH: &str = "world_state.json";
const WORLD_STATE_URL: &str = "https://content.warframe.com/dynamic/worldState.php";

/// Persistently keeps track when the data was last updated.
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub struct Storage {
    /// How many seconds to wait before fetching new data.
    pub update_cooldown: i64,
    /// When the last fetch happened in seconds.
    pub last_update: i64,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            update_cooldown: 300,
            last_update: 0,
        }
    }
}

impl Storage {
    /// Try to decode data file contents into `Storage`.
    /// Creates a new file if it does not exist.
    pub fn from_file(file: &str) -> Self {
        let path = current_dir().unwrap();
        let file_path = path.join(file);

        // Create the storage file if it does not exist
        if !&file_path.exists() {
            let data = Self::default();
            // Create new storage file
            data.write_to_file().expect("Cannot create storage file!");

            return data;
        }

        // Open the storage file and try to decode it
        match File::open(&file_path) {
            Ok(mut f) => match decode_from_std_read(&mut f, config::standard()) {
                Ok(s) => s,
                Err(e) => panic!("{}", e),
            },
            Err(e) => panic!("{}", e),
        }
    }

    /// Encode and write to file.
    pub fn write_to_file(&self) -> Result<usize, bincode::error::EncodeError> {
        let path = current_dir().unwrap();
        let file_path = path.join(STORAGE_FILE);
        let f = File::create(&file_path).expect("Cannot create file!");
        let mut writer = BufWriter::new(f);

        encode_into_std_write(self, &mut writer, config::standard())
    }

    /// Returns true if enough time has passed since the last update.
    pub fn can_update(&self) -> bool {
        self.last_update + self.update_cooldown < Local::now().timestamp()
    }

    /// Next update can happen in this many seconds. Debug use.
    pub fn next_update(&self) -> i64 {
        (self.last_update + self.update_cooldown) - Local::now().timestamp()
    }
}

/// Message for cross thread sending & receiving.
enum Message {
    /// Send when the initial data has loaded, likely from the local files.
    Initialized,
    /// Send when new update (from url) happened.
    Updated,
}

/// Contains all the data the UI needs.
#[derive(Debug, Clone)]
pub struct TennoData {
    /// True when all the initial data has loaded.
    pub initialized: bool,
    /// List of Fissures, Void Storms included.
    pub fissures: Vec<Fissure>,
    /// Cetus cycle data.
    pub cetus_cycle: CetusCycle,

    pub storage: Storage,
}

impl Default for TennoData {
    fn default() -> Self {
        Self {
            initialized: false,
            fissures: vec![],
            cetus_cycle: Default::default(),
            storage: Storage::from_file(STORAGE_FILE),
        }
    }
}

/// The actual app.
#[derive(Debug)]
pub struct VoidRat {
    /// All the data the UI needs. Thread safe.
    pub data: Arc<RwLock<TennoData>>,
    /// A cool loop handle (seems the `l` killed a dash).
    _loop: JoinHandle<()>,
}

impl Default for VoidRat {
    fn default() -> Self {
        Self::new()
    }
}

impl VoidRat {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Message>();
        let tenno_data = TennoData::default();

        let data = Arc::new(RwLock::new(tenno_data));
        let data_clone = data.clone();
        let _loop = thread::spawn(move || Self::event_loop(data_clone, tx, rx));

        VoidRat { data, _loop }
    }

    /// Loop for all the things.
    ///
    /// Loads the initial data upon app startup.
    ///
    /// Handles updating the existing data periodically.
    fn event_loop(data: Arc<RwLock<TennoData>>, tx: Sender<Message>, rx: Receiver<Message>) {
        let mut initialized = false;
        let mut updating = false;

        loop {
            if let Ok(msg) = rx.try_recv() {
                match msg {
                    Message::Initialized => {
                        data.write().initialized = true;
                        initialized = true;
                    }
                    Message::Updated => {
                        // Data was updated, update the time and save to file.
                        data.write().storage.last_update = Local::now().timestamp();
                        data.write()
                            .storage
                            .write_to_file()
                            .expect("Cannot write to storage file!");
                        // Set `updating` false since everything is done.
                        updating = false;

                        debug!("Updated!");
                    }
                }
            }

            if !initialized {
                // INITIALIZE
                //
                let data_path = PathBuf::from(DATA_PATH);
                let world_state_file = &data_path.join(WORLD_STATE_DATA_PATH);
                let fissure_file = &data_path.join("fissure.json");
                let cetus_file = &data_path.join("cetus.json");

                if !data_path.exists() {
                    create_dir(&data_path).expect("Cannot create the data directory.");
                }

                if !world_state_file.exists() {
                    if let Some(world_data) = fetch_json_data(WORLD_STATE_URL) {
                        fs::write(&world_state_file, world_data)
                            .expect("Unable to write world state file.");

                        data.write().storage.last_update = Local::now().timestamp();
                    }
                }

                if fissure_file.exists()
                    && cetus_file.exists()
                    && FileTime::from_last_modification_time(&fs::metadata(fissure_file).unwrap())
                        > FileTime::from_last_modification_time(
                            &fs::metadata(world_state_file).unwrap(),
                        )
                {
                    let p = WarframeStat {};
                    let fissure_data = fs::read_to_string(fissure_file)
                        .expect("Something went wrong reading the file.");
                    let f = p.parse_fissures(&fissure_data);
                    data.write().fissures = f;

                    let cetus_data = fs::read_to_string(cetus_file)
                        .expect("Something went wrong reading the file.");
                    let c = p.parse_cetus_cycle(&cetus_data);
                    data.write().cetus_cycle = c;

                    tx.send(Message::Initialized)
                        .expect("Cannot send initialized msg!");
                } else {
                    let p = WorldState {};

                    let world_state_data =
                        match fs::read_to_string(&data_path.join(WORLD_STATE_DATA_PATH)) {
                            Ok(d) => d,
                            Err(e) => panic!("{}", e),
                        };

                    data.write().fissures = p.parse_fissures(&world_state_data);
                    data.write().cetus_cycle = p.parse_cetus_cycle(&world_state_data);

                    tx.send(Message::Initialized)
                        .expect("Cannot send initialized msg!");
                }
            }

            // UPDATE
            //
            debug!("Next update in: {:?}", data.read().storage.next_update());

            if data.read().storage.can_update() && !updating {
                updating = true;

                debug!("Updating..");

                let tx_clone = tx.clone();
                let data_clone = data.clone();
                thread::spawn(move || {
                    let parser = WorldState {};

                    if let Some(json) = fetch_json_data(WORLD_STATE_URL) {
                        let file_path = PathBuf::from(DATA_PATH).join("world_state.json");
                        fs::write(&file_path, json.clone()).expect("Cannot write to file.");

                        data_clone.write().fissures = parser.parse_fissures(&json);
                        data_clone.write().cetus_cycle = parser.parse_cetus_cycle(&json);

                        tx_clone
                            .send(Message::Updated)
                            .expect("Cannot send updated msg!");
                    } else {
                        warn!("Failed to fetch json data from primary source, using fallback instead.");
                        // Since worldState failed for some reason try to use warframestat as a fallback.
                        let fallback = WarframeStat {};

                        if let Some(json) =
                            fetch_json_data("https://api.warframestat.us/pc/fissures")
                        {
                            let file_path = PathBuf::from(DATA_PATH).join("fissure.json");
                            fs::write(&file_path, json.clone()).expect("Cannot write to file.");

                            let f = fallback.parse_fissures(&json);

                            data_clone.write().fissures = f;

                            tx_clone
                                .send(Message::Updated)
                                .expect("Cannot send updated msg!");
                        }

                        if let Some(json) =
                            fetch_json_data("https://api.warframestat.us/pc/cetusCycle")
                        {
                            let file_path = PathBuf::from(DATA_PATH).join("cetus.json");
                            fs::write(&file_path, json.clone()).expect("Cannot write to file.");

                            let c = fallback.parse_cetus_cycle(&json);

                            data_clone.write().cetus_cycle = c;

                            tx_clone
                                .send(Message::Updated)
                                .expect("Cannot send updated msg!");
                        }
                    }
                });
            }

            // Take a quick nap.
            thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}

/// Might return json string from url.
fn fetch_json_data(url: &str) -> Option<String> {
    debug!("Fetching {}", url);

    let res = reqwest::blocking::get(url).ok()?;

    // Only write to the file if status is a success.
    if res.status().is_success() {
        return res.text().ok();
    }

    None
}

impl Fissure {
    /// Returns a `Duration` of time till fissure expires.
    pub fn till_expired(&self) -> Duration {
        let now: DateTime<Utc> = Utc::now();
        self.expiry - now
    }

    /// Returns true if the fissure has expired.
    pub fn has_expired(&self) -> bool {
        let now: DateTime<Utc> = Utc::now();
        self.expiry < now
    }
}

impl CetusCycle {
    /// Get Cetus Day / Night cycle status based on the bounties activation and expiry times.
    /// Day cycle should be 100 minutes and night cycle 50 minutes.
    pub fn cetus_is_day(&self) -> bool {
        let _day_time = 6000;
        let night_time = 3000;

        let now = Utc::now().timestamp();
        let millis_left = self.expiry.timestamp() - now;

        millis_left >= night_time
    }

    /// Returns `Duration` of time till current cycle ends.
    pub fn cetus_till_cycle(&self) -> Duration {
        let _day_time = 6000;
        let night_time = 3000;

        let now = Utc::now();

        let night_start = self.expiry - Duration::seconds(night_time);

        let total_time_left = self.expiry - now;
        if (night_start - now).num_seconds() > 0 {
            night_start - now
        } else {
            total_time_left
        }
    }
}

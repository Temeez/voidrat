use crate::parsers::world_state::WorldState;
use crate::parsers::{CetusCycle, Fissure, Invasion, Reward, TennoParser};

use bincode::{config, decode_from_std_read, encode_into_std_write};
use chrono::{DateTime, Duration, Local, Utc};
use log::{debug, warn};
use parking_lot::RwLock;

use std::env::current_dir;
use std::fs::{create_dir, File};
use std::io::{BufWriter, Cursor, Seek, SeekFrom, Write};

use std::path::PathBuf;

use crate::parsers::warframestat::WarframeStat;
use crate::Resources;
use filetime::FileTime;
use rodio::{Decoder, OutputStream, Source};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::{fs, thread};

const STORAGE_FILE: &str = "voidrat.storage";
const DATA_PATH: &str = "data";
const WORLD_STATE_DATA_PATH: &str = "world_state.json";
const WORLD_STATE_URL: &str = "https://content.warframe.com/dynamic/worldState.php";

#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub struct Notification {
    pub timestamp: i64,
}

impl Notification {
    pub fn new(timestamp: i64) -> Self {
        Notification { timestamp }
    }
}

/// Persistently keeps track when the data was last updated.
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub struct Storage {
    /// How many seconds to wait before fetching new data.
    pub update_cooldown: i64,
    /// When the last fetch happened in seconds.
    pub last_update: i64,

    pub notified: Vec<Notification>,

    pub noti_fissure_void_capture: bool,
    pub noti_invasion_epic: bool,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            update_cooldown: 300,
            last_update: 0,
            notified: vec![],
            noti_fissure_void_capture: false,
            noti_invasion_epic: false,
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

        debug!("Writing to file..");

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

    pub fn save_notification(&mut self, a: bool, b: bool) {
        self.noti_fissure_void_capture = a;
        self.noti_invasion_epic = b;

        self.write_to_file().expect("Cannot write to storage file.");
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
    /// Invasions
    pub invasions: Vec<Invasion>,

    pub storage: Storage,
}

impl Default for TennoData {
    fn default() -> Self {
        Self {
            initialized: false,
            fissures: vec![],
            cetus_cycle: Default::default(),
            invasions: vec![],
            storage: Storage::from_file(STORAGE_FILE),
        }
    }
}

impl TennoData {
    /// Returns true if any of the invasion rewards contain
    /// a forma, orokin reactor or orokin catalyst.
    pub fn has_epic_invasion(&self) -> Option<Invasion> {
        self.invasions
            .iter()
            .find(|i| {
                ["forma", "reactor", "catalyst"]
                    .iter()
                    .any(|w| i.rewards.all_rewards_string().to_lowercase().contains(w))
            })
            .map(|i| i.to_owned())
    }

    /// Returns true if one of the active fissures is in the Void with Capture map.
    pub fn has_void_capture(&self) -> Option<Fissure> {
        self.fissures
            .iter()
            .find(|f| {
                !f.is_storm && (f.node.value == "Hepit (Void)" || f.node.value == "Ukko (Void)")
            })
            .map(|f| f.to_owned())
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
                            .expect("Cannot write to storage file.");
                        // Set `updating` false since everything is done.
                        updating = false;

                        //
                        // Play notification if maybe perhaps
                        //
                        let mut new_noti = false;
                        let mut storage = data.read().storage.clone();
                        let old_notis = data.read().storage.notified.clone();
                        // Fissure notifications
                        if storage.noti_fissure_void_capture {
                            if let Some(fissure) = data.read().has_void_capture() {
                                if !old_notis
                                    .iter()
                                    .any(|n| n.timestamp == fissure.activation.timestamp())
                                {
                                    play_notification_sound();

                                    storage
                                        .notified
                                        .push(Notification::new(fissure.activation.timestamp()));

                                    new_noti = true;
                                }
                            }
                        }
                        // Invasion notifications
                        if storage.noti_invasion_epic {
                            if let Some(invasion) = data.read().has_epic_invasion() {
                                if !old_notis
                                    .iter()
                                    .any(|n| n.timestamp == invasion.activation.timestamp())
                                {
                                    play_notification_sound();

                                    storage
                                        .notified
                                        .push(Notification::new(invasion.activation.timestamp()));

                                    new_noti = true;
                                }
                            }
                        }

                        if new_noti {
                            data.write().storage = storage;
                            data.write()
                                .storage
                                .write_to_file()
                                .expect("Cannot write to storage file.");
                        }

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
                let invasion_file = &data_path.join("invasion.json");

                // Create the data directory if it does not exist.
                if !data_path.exists() {
                    create_dir(&data_path).expect("Cannot create the data directory.");
                }

                // If world state date file is missing,
                // then get the data from url and
                // create the file with the new data.
                if !world_state_file.exists() {
                    if let Some(world_data) = fetch_json_data(WORLD_STATE_URL) {
                        fs::write(&world_state_file, world_data)
                            .expect("Unable to write world state file.");

                        data.write().storage.last_update = Local::now().timestamp();
                    }
                }

                // Load data from local files if they exist and they are newer
                // than the world state data file.
                // Otherwise get data from world state data file.
                if fissure_file.exists()
                    && cetus_file.exists()
                    && invasion_file.exists()
                    && FileTime::from_last_modification_time(&fs::metadata(fissure_file).unwrap())
                        > FileTime::from_last_modification_time(
                            &fs::metadata(world_state_file).unwrap(),
                        )
                {
                    let p = WarframeStat {};
                    // Fissure data
                    let fissure_data = fs::read_to_string(fissure_file)
                        .expect("Something went wrong reading the file.");
                    data.write().fissures = p.parse_fissures(&fissure_data);
                    // Cetus cycle data
                    let cetus_data = fs::read_to_string(cetus_file)
                        .expect("Something went wrong reading the file.");
                    data.write().cetus_cycle = p.parse_cetus_cycle(&cetus_data);
                    // Invasion data
                    let invasion_data = fs::read_to_string(invasion_file)
                        .expect("Something went wrong reading the file.");
                    data.write().invasions = p.parse_invasions(&invasion_data);

                    tx.send(Message::Initialized)
                        .expect("Cannot send initialized msg!");
                } else {
                    // Load data from the world state file, that definitely exists.
                    let p = WorldState {};

                    let world_state_data =
                        match fs::read_to_string(&data_path.join(WORLD_STATE_DATA_PATH)) {
                            Ok(d) => d,
                            Err(e) => panic!("{}", e),
                        };

                    data.write().fissures = p.parse_fissures(&world_state_data);
                    data.write().cetus_cycle = p.parse_cetus_cycle(&world_state_data);
                    data.write().invasions = p.parse_invasions(&world_state_data);

                    tx.send(Message::Initialized)
                        .expect("Cannot send initialized msg!");
                }
            }

            // UPDATE
            //
            debug!("Next update in: {:?}", data.read().storage.next_update());

            if data.read().storage.can_update() && !updating {
                // Started updating, let us not do this every tick, heh.
                updating = true;

                debug!("Updating..");

                let tx_clone = tx.clone();
                let data_clone = data.clone();
                //
                // New thread
                //
                thread::spawn(move || {
                    // Parse data from world state data, fresh from the oven (net).
                    let parser = WorldState {};

                    if let Some(json) = fetch_json_data(WORLD_STATE_URL) {
                        let file_path = PathBuf::from(DATA_PATH).join("world_state.json");
                        // Got cool json data so put it in the local file for easy re-use.
                        fs::write(&file_path, json.clone()).expect("Cannot write to file.");

                        data_clone.write().fissures = parser.parse_fissures(&json);
                        data_clone.write().cetus_cycle = parser.parse_cetus_cycle(&json);
                        data_clone.write().invasions = parser.parse_invasions(&json);

                        tx_clone
                            .send(Message::Updated)
                            .expect("Cannot send updated msg!");
                    } else {
                        // Since worldState failed for some reason try to use warframestat as a fallback.
                        warn!("Failed to fetch json data from primary source, using fallback instead.");

                        let fallback = WarframeStat {};

                        if let Some(json) =
                            fetch_json_data("https://api.warframestat.us/pc/fissures")
                        {
                            let file_path = PathBuf::from(DATA_PATH).join("fissure.json");
                            fs::write(&file_path, json.clone()).expect("Cannot write to file.");

                            data_clone.write().fissures = fallback.parse_fissures(&json);

                            tx_clone
                                .send(Message::Updated)
                                .expect("Cannot send updated msg!");
                        }

                        if let Some(json) =
                            fetch_json_data("https://api.warframestat.us/pc/cetusCycle")
                        {
                            let file_path = PathBuf::from(DATA_PATH).join("cetus.json");
                            fs::write(&file_path, json.clone()).expect("Cannot write to file.");

                            data_clone.write().cetus_cycle = fallback.parse_cetus_cycle(&json);

                            tx_clone
                                .send(Message::Updated)
                                .expect("Cannot send updated msg!");
                        }

                        if let Some(json) =
                            fetch_json_data("https://api.warframestat.us/pc/invasions")
                        {
                            let file_path = PathBuf::from(DATA_PATH).join("invasion.json");
                            fs::write(&file_path, json.clone()).expect("Cannot write to file.");

                            data_clone.write().invasions = fallback.parse_invasions(&json);

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

impl Invasion {
    pub fn active_duration(&self) -> Duration {
        let now = Utc::now();

        now - self.activation
    }
}

impl ToString for Reward {
    fn to_string(&self) -> String {
        if self.quantity > 1 {
            return format!("{} {}", self.quantity, self.item);
        }

        self.item.clone()
    }
}

pub fn play_notification_sound() {
    // Get a output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    // Load a sound from a file, using a path relative to Cargo.toml

    // "In memory file"
    let mut c = Cursor::new(Vec::new());
    c.write_all(&Resources::get("audio/notification.wav").unwrap().data)
        .unwrap();
    c.seek(SeekFrom::Start(0)).unwrap();

    // Decode that sound file into a source
    let source = Decoder::new(c).unwrap();
    // Play the sound directly on the device
    stream_handle
        .play_raw(source.convert_samples())
        .expect("Error playing notification.wav!");

    // The sound plays in a separate audio thread,
    // so we need to keep the main thread alive while it's playing.
    // Audio file has a duration of second or less.
    thread::sleep(std::time::Duration::from_secs(1));
}

/// The file containing the SHA256 hash of all other files.
pub const HASH: &str = "_hash";

/// The file containing the PKCS#1 v1.5 signature for the hash.
pub const SIG: &str = "_sig";

/// The WebAssembly binary with the app.
pub const BIN: &str = "_bin";

/// The metadata file with all the basic info about the app: name, version, author, etc.
pub const META: &str = "_meta";

/// The public key that can verify the author's signature.
pub const KEY: &str = "_key";

/// Description of badges (aka achievements) provided by the app.
pub const BADGES: &str = "_badges";

/// Description of boards (aka scoreboards or leaderboards) provided by the app.
pub const BOARDS: &str = "_boards";

/// The default stats file.
///
/// It is used to create a new stats file in the data directory
/// when installing a new app or after cleaning the app data,
/// or to update the existing stats file when the app is updated.
pub const STATS: &str = "_stats";

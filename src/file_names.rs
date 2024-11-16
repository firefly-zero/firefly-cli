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

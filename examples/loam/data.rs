//! The canned garden. Everything lives in memory for one run; nothing is read from or
//! written to disk.

/// A seed in the catalogue.
pub struct Seed {
    pub name: &'static str,
    /// When to sow it.
    pub season: &'static str,
    /// Days until it sprouts, from and to.
    pub sprouts: (u8, u8),
}

impl Seed {
    /// `7-10 days`.
    pub fn sprouts_in(&self) -> String {
        format!("{}-{} days", self.sprouts.0, self.sprouts.1)
    }
}

/// The 40-seed catalogue.
#[rustfmt::skip]
pub const SEEDS: [Seed; 40] = [
    Seed { name: "basil", season: "spring", sprouts: (7, 10) },
    Seed { name: "tomato", season: "spring", sprouts: (6, 10) },
    Seed { name: "zucchini", season: "spring", sprouts: (5, 8) },
    Seed { name: "pumpkin", season: "spring", sprouts: (6, 10) },
    Seed { name: "cucumber", season: "spring", sprouts: (5, 9) },
    Seed { name: "runner bean", season: "spring", sprouts: (7, 14) },
    Seed { name: "pea", season: "spring", sprouts: (7, 14) },
    Seed { name: "carrot", season: "spring", sprouts: (10, 21) },
    Seed { name: "beetroot", season: "spring", sprouts: (7, 14) },
    Seed { name: "radish", season: "spring", sprouts: (3, 7) },
    Seed { name: "lettuce", season: "spring", sprouts: (4, 10) },
    Seed { name: "spinach", season: "autumn", sprouts: (7, 14) },
    Seed { name: "kale", season: "summer", sprouts: (5, 8) },
    Seed { name: "chard", season: "spring", sprouts: (7, 14) },
    Seed { name: "leek", season: "spring", sprouts: (10, 14) },
    Seed { name: "onion", season: "spring", sprouts: (7, 12) },
    Seed { name: "garlic", season: "autumn", sprouts: (14, 21) },
    Seed { name: "parsley", season: "spring", sprouts: (14, 28) },
    Seed { name: "coriander", season: "spring", sprouts: (7, 14) },
    Seed { name: "dill", season: "spring", sprouts: (10, 14) },
    Seed { name: "chives", season: "spring", sprouts: (10, 14) },
    Seed { name: "mint", season: "spring", sprouts: (10, 15) },
    Seed { name: "thyme", season: "spring", sprouts: (14, 28) },
    Seed { name: "sage", season: "spring", sprouts: (10, 21) },
    Seed { name: "rosemary", season: "spring", sprouts: (14, 28) },
    Seed { name: "fennel", season: "spring", sprouts: (7, 14) },
    Seed { name: "borage", season: "spring", sprouts: (5, 10) },
    Seed { name: "marigold", season: "spring", sprouts: (5, 8) },
    Seed { name: "nasturtium", season: "spring", sprouts: (7, 12) },
    Seed { name: "calendula", season: "spring", sprouts: (7, 14) },
    Seed { name: "sunflower", season: "spring", sprouts: (7, 10) },
    Seed { name: "sweet pea", season: "autumn", sprouts: (10, 21) },
    Seed { name: "cosmos", season: "spring", sprouts: (7, 10) },
    Seed { name: "strawberry", season: "spring", sprouts: (14, 28) },
    Seed { name: "pepper", season: "spring", sprouts: (10, 21) },
    Seed { name: "aubergine", season: "spring", sprouts: (10, 14) },
    Seed { name: "sweetcorn", season: "spring", sprouts: (7, 10) },
    Seed { name: "turnip", season: "summer", sprouts: (4, 7) },
    Seed { name: "parsnip", season: "spring", sprouts: (14, 28) },
    Seed { name: "celery", season: "spring", sprouts: (14, 21) },
];

/// The index of `name` in [`SEEDS`].
pub fn seed_index(name: &str) -> Option<usize> {
    SEEDS.iter().position(|s| s.name == name)
}

/// A plant that grows well beside another.
pub struct Companion {
    pub name: &'static str,
    pub why: &'static str,
    /// Checked when the question opens.
    pub checked: bool,
    pub in_season: bool,
}

#[rustfmt::skip]
pub const COMPANIONS: [Companion; 6] = [
    Companion { name: "marigold", why: "keeps aphids away", checked: true, in_season: true },
    Companion { name: "nasturtium", why: "catches the aphids instead", checked: true, in_season: true },
    Companion { name: "chives", why: "shoos the carrot fly", checked: false, in_season: true },
    Companion { name: "borage", why: "calls the bees", checked: false, in_season: true },
    Companion { name: "calendula", why: "a cheerful cover", checked: false, in_season: true },
    Companion { name: "pumpkin", why: "out of season", checked: false, in_season: false },
];

/// A climate zone.
pub struct Zone {
    pub name: &'static str,
    pub about: &'static str,
    pub open: bool,
}

#[rustfmt::skip]
pub const ZONES: [Zone; 6] = [
    Zone { name: "temperate", about: "mild winters, long summers", open: true },
    Zone { name: "cool", about: "cold winters, short summers", open: true },
    Zone { name: "mediterranean", about: "hot dry summers, wet winters", open: true },
    Zone { name: "tropical", about: "warm and wet all year", open: true },
    Zone { name: "arid", about: "scorching days, cold nights", open: true },
    Zone { name: "polar", about: "coming soon", open: false },
];

/// Names the neighbours already use.
pub const TAKEN_NAMES: [&str; 5] = [
    "the allotment",
    "sunny corner",
    "south bed",
    "herb spiral",
    "the cold frame",
];

pub fn is_taken(name: &str) -> bool {
    let name = name.trim();
    TAKEN_NAMES.iter().any(|t| t.eq_ignore_ascii_case(name))
}

/// A bed of the garden.
pub struct Bed {
    pub name: &'static str,
    pub plants: &'static str,
    pub watered: &'static str,
    /// Soil moisture, in percent.
    pub moisture: u8,
    /// Litres it drinks.
    pub litres: u8,
}

impl Bed {
    /// Below 40% a bed is dry.
    pub fn is_dry(&self) -> bool {
        self.moisture < 40
    }
}

#[rustfmt::skip]
pub const BEDS: [Bed; 6] = [
    Bed { name: "1 south bed", plants: "tomatoes, basil", watered: "yesterday", moisture: 62, litres: 8 },
    Bed { name: "2 herb spiral", plants: "thyme, sage, mint", watered: "3 days ago", moisture: 31, litres: 5 },
    Bed { name: "3 seedling tray", plants: "lettuce, leek", watered: "today", moisture: 74, litres: 2 },
    Bed { name: "4 salad rows", plants: "radish, rocket", watered: "2 days ago", moisture: 48, litres: 6 },
    Bed { name: "5 bean teepee", plants: "runner beans", watered: "9 days ago", moisture: 18, litres: 10 },
    Bed { name: "6 cold frame", plants: "spinach, chard", watered: "yesterday", moisture: 55, litres: 4 },
];

/// A day of the week's weather.
pub struct Day {
    pub name: &'static str,
    pub outlook: &'static str,
    /// The night's low, in degrees.
    pub low: i8,
}

#[rustfmt::skip]
pub const WEEK: [Day; 7] = [
    Day { name: "monday", outlook: "frost tonight", low: -1 },
    Day { name: "tuesday", outlook: "clear", low: 3 },
    Day { name: "wednesday", outlook: "clouds", low: 5 },
    Day { name: "thursday", outlook: "rain", low: 7 },
    Day { name: "friday", outlook: "drizzle", low: 6 },
    Day { name: "saturday", outlook: "sun", low: 4 },
    Day { name: "sunday", outlook: "sun", low: 3 },
];

/// Days to the first autumn frost.
pub const FIRST_FROST_DAYS: u8 = 12;

/// A crop that is ready to pick.
pub struct Crop {
    pub name: &'static str,
    pub kilos: f32,
    /// Left too long on the vine.
    pub overripe: bool,
}

#[rustfmt::skip]
pub const CROPS: [Crop; 5] = [
    Crop { name: "tomatoes", kilos: 1.2, overripe: false },
    Crop { name: "zucchini", kilos: 2.4, overripe: true },
    Crop { name: "runner beans", kilos: 0.6, overripe: false },
    Crop { name: "lettuce", kilos: 0.4, overripe: false },
    Crop { name: "basil", kilos: 0.1, overripe: false },
];

/// Old journal notes, oldest first.
pub const JOURNAL: [&str; 5] = [
    "sowed the first radishes in bed 4",
    "the marigolds are up beside the tomatoes",
    "rain all day, nothing to do but watch it",
    "first runner bean flowers on the teepee",
    "picked the lettuce before the slugs did",
];

/// What `loam status --json` prints.
pub const STATUS_JSON: &str = r#"{"garden":"the back plot","zone":"temperate","beds":6,"plants":14,"frost_tonight":true,"needs_attention":["5 bean teepee"]}"#;

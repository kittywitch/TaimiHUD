use {
    crate::timer::{
        TimerPhase,
        TimerTrigger
    }, relative_path::RelativePathBuf, serde::{Deserialize, Serialize}, std::path::PathBuf
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerFile {
    #[serde(default,skip)]
    pub path: Option<PathBuf>,
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub author: String,
    pub icon: RelativePathBuf,
    // I probably don't need to do this, but it's convenient :o
    #[serde(rename = "map")]
    pub map_id: u32,
    pub reset: TimerTrigger,
    pub phases: Vec<TimerPhase>,
}

impl TimerFile {
    pub fn name(&self) -> String {
        self.name.replace("\n", " ")
    }
    pub fn title(&self) -> &str {
        self.name.split('\n').next().unwrap()
    }
    pub fn subtitle(&self) -> Option<String> {
        Some(self.name.split_once('\n')?.1.replace("\n", " - "))
    }
    pub fn combined(&self) -> String {
        match self.subtitle() {
            Some(sbubby) => format!("{}\n{}", self.title(), sbubby),
            None => self.name.clone()
        }

    }
    pub fn hypheny_name(&self) -> String {
        self.name.replace("\n", " - ")
    }
    pub fn author(&self) -> String {
        self.author.replace("\n", "")
    }
}


use {
    crate::timer::{
        TimerPhase,
        TimerTrigger
    },
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerFile {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub author: String,
    pub icon: String,
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
    pub fn hypheny_name(&self) -> String {
        self.name.replace("\n", " - ")
    }
    pub fn author(&self) -> String {
        self.author.replace("\n", "")
    }
}

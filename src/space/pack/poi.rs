use {
    super::{
        attributes::MarkerAttributes, loader::PackLoaderContext, taco_safe_name, taco_xml_to_guid,
        Pack,
    },
    crate::marker::atomic::MapSpace,
    anyhow::Context,
    glamour::Vector3,
    uuid::Uuid,
};

pub struct Poi {
    pub category: String,
    pub guid: Uuid,
    pub map_id: i32,
    pub position: Vector3<MapSpace>,
    pub attributes: MarkerAttributes,
}

impl Poi {
    pub fn from_xml(
        pack: &mut Pack,
        attrs: Vec<xml::attribute::OwnedAttribute>,
    ) -> anyhow::Result<Poi> {
        let mut category = String::new();
        let mut map_id = None;
        let mut pos_x = None;
        let mut pos_y = None;
        let mut pos_z = None;
        let mut guid = None;
        let mut attributes = MarkerAttributes::default();

        for attr in attrs {
            if attr.name.local_name.eq_ignore_ascii_case("type") {
                category = taco_safe_name(&attr.value, true);
            } else if attr.name.local_name.eq_ignore_ascii_case("MapID") {
                map_id = Some(attr.value.parse().context("Parse POI MapID")?);
            } else if attr.name.local_name.eq_ignore_ascii_case("xpos") {
                pos_x = Some(attr.value.parse().context("Parse POI xpos")?);
            } else if attr.name.local_name.eq_ignore_ascii_case("ypos") {
                pos_y = Some(attr.value.parse().context("Parse POI ypos")?);
            } else if attr.name.local_name.eq_ignore_ascii_case("zpos") {
                pos_z = Some(attr.value.parse().context("Parse POI zpos")?);
            } else if attr.name.local_name.eq_ignore_ascii_case("guid") {
                guid = Some(taco_xml_to_guid(&attr.value));
            } else if !attributes.try_add(pack, &attr) {
                log::warn!("Unknown POI attribute '{}'", attr.name.local_name);
            }
        }

        let Some(map_id) = map_id else {
            anyhow::bail!("POI must have MapID");
        };

        let (Some(pos_x), Some(pos_y), Some(pos_z)) = (pos_x, pos_y, pos_z) else {
            anyhow::bail!("POI must have xpos, ypos, and zpos");
        };
        let position = glamour::vec3!(pos_x, pos_y, pos_z);

        let guid = guid.unwrap_or_default();

        Ok(Poi {
            category,
            guid,
            map_id,
            position,
            attributes,
        })
    }
}

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Club {
    pub id: u32,
    pub uid: u32,
    pub name: String,
    pub short_name: String,
    pub nation: u32,
    pub offset: usize,
}

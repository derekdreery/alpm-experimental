

#[derive(Serialize, Deserialize)]
pub struct Package {
    name: String,
    version: String,
    #[serde(rename = "desc")]
    description: String,
    url: String,
    arch: String,
    build_date: u64,
    install_date: u64,
    packager: String,
    size: u64,
    reason: u8,
    license: Vec<String>,
    validation: String,
    depends: Vec<String>
}
//! Event references and story flags for this map; original selection metadata.
pub const PROFILE: &str = r#"
{
  "seed_rules": [{"when":[8306],"set":[9810]}],
  "sub_files": {"30":"m11_02_0930"},
  "default": "prologue",
  "map": "m11_02",
  "event_sub_files": {"20":"m11_02_0920","30":"m11_02_0930"},
  "states": [
    {"flags":[8306,9810],"name":"prologue"},
    {"flags":[],"name":"morning"},
    {"flags":[9800],"name":"afternoon"},
    {"flags":[9800,9801],"name":"evening"},
    {"flags":[9800,9801,9802],"name":"night"},
    {"flags":[9800,9801,9802,9803],"name":"dawn"},
    {"flags":[9800,9801,9802,8302],"name":"invasion"},
    {"commands":[[11125860,19]],"flags":[9800,9801,9802,8302,9312],"name":"sword-saint-defeated"},
    {"commands":[[11125872,12]],"flags":[9800,9801,9802,8302],"name":"sword-saint-storm"}
  ]
}
"#;

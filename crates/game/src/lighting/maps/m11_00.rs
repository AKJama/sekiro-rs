//! Event references and story flags for this map; original selection metadata.
pub const PROFILE: &str = r#"
{
  "default": "morning",
  "map": "m11_00",
  "states": [
    {"flags":[],"name":"morning"},
    {"flags":[9800],"name":"afternoon"},
    {"flags":[9800,9801],"name":"evening"},
    {"flags":[9800,9801,9802],"name":"night"},
    {"flags":[9800,9801,9802,9803],"name":"dawn"},
    {"flags":[9800,9801,9802,8302],"name":"invasion"},
    {"commands":[[11115820,24],[11115820,26],[11115820,28]],"flags":[],"name":"tomoe","event_map":"m11_01_00_00"}
  ]
}
"#;

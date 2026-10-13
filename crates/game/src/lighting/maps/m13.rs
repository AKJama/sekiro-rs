//! Event references and story flags for this map; original selection metadata.
pub const PROFILE: &str = r#"
{
  "default": "morning",
  "map": "m13_00",
  "states": [
    {"flags":[],"name":"morning"},
    {"flags":[9800],"name":"afternoon"},
    {"flags":[9800,9801],"name":"evening"},
    {"flags":[9800,9801,9802],"name":"night"},
    {"flags":[9800,9801,9802,9803],"name":"dawn"}
  ]
}
"#;

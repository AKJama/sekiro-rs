//! Event references and story flags for this map; original selection metadata.
pub const PROFILE: &str = r#"
{
  "default": "morning",
  "map": "m20_00",
  "states": [
    {"flags":[],"name":"morning"},
    {"flags":[9800],"name":"afternoon"},
    {"flags":[9800,9801],"name":"evening"},
    {"flags":[9800,9801,9802],"name":"night"},
    {"flags":[9800,9801,9802,9803],"name":"dawn"},
    {"name":"illusion","flags":[8400,9303],"when_flags":[8400],"commands":[[12005210,9],[12005210,25]],"events":[12005826]},
    {"name":"illusion-cleared","flags":[8400,9303,62000705],"commands":[[12005855,12]]}
  ]
}
"#;

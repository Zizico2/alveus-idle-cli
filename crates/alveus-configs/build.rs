use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use alveus_types::EnclosureId;

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("poop_config.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    writeln!(
        file,
        "pub static POOP_CONFIG: phf::Map<alveus_types::EnclosureId, PoopConfig> = {};",
        phf_codegen::Map::<EnclosureId>::new()
            .entry(
                EnclosureId::PushPopEnclosure,
                "PoopConfig {
                    spawn_thresholds: &[800, 500, 200],
                    poop_decay_rate: 20.0,
                    cleanliness_restore_per_poop: 350,
                    spawn_bounds: PUSH_POP_PLACEMENT.wander_bounds,
                }",
            )
            .build()
    )
    .unwrap();
}

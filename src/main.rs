use natural_tts::{models::gtts::GttsModel, NaturalTtsBuilder};
fn main() {
    let mut natural = NaturalTtsBuilder::default()
        .gtts_model(GttsModel::default())
        .default_model(Model::GttsModel)
        .build()
        .unwarp();
}

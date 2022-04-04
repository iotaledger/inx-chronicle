use mongodb::{bson, bson::Bson};


#[derive()]
pub enum MessageId {
    Chrysalis(bee_message_chrysalis::MessageId),
    Stardust(bee_message_stardust::MessageId),
}

fn conv_bson(id: &MessageId) -> Bson {
    bson::to_bson(id)
}



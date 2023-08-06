use chrono::{DateTime, Utc};
use diesel::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    time::SystemTime,
};
use uuid::Uuid;
use crate::models::{Conversation, NewConversation, Room, RoomResponse, User};
type DbError = Box<dyn std::error::Error + Send + Sync>;

fn iso_date() -> String {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    return now.to_rfc3339();
}

pub fn find_user_by_uid(conn: &mut SqliteConnection, uid: Uuid) -> Result<Option<User>, DbError> {
    use crate::schema::users::dsl::*;

    let user = users
        .filter(id.eq(uid.to_string()))
        .first::<User>(conn)
        .optional()?;

    Ok(user)
}

pub fn get_conversation_by_room_uid(
    conn: &mut SqliteConnection,
    uid: Uuid,
) -> Result<Option<Vec<Conversation>>, DbError> {
    use crate::schema::conversations;

    let convo = conversations::table
        .filter(conversations::room_id.eq(uid.to_string()))
        .load(conn)
        .optional()?;

    Ok(convo)
}

pub fn find_user_by_phone(
    conn: &mut SqliteConnection,
    user_phone: String,
) -> Result<Option<User>, DbError> {
    use crate::schema::users::dsl::*;
    let user = users
        .filter(phone.eq(user_phone))
        .first::<User>(conn)
        .optional()?;
    Ok(user)
}

pub fn update_room_user(
    conn: &mut SqliteConnection,
    new_conversation: &NewConversation,
) -> Result<Option<Room>, DbError> {
    use crate::schema::rooms;
    use crate::schema::users;
    let room = rooms::table
        .filter(rooms::id.eq(&new_conversation.room_id))
        .first::<Room>(conn)
        .optional()?;

    let binding = users::table
        .filter(users::id.eq(&new_conversation.user_id))
        .first::<User>(conn)
        .optional()?;
    let user: &User = binding.as_ref().unwrap();

    let user_ids = &mut room.as_ref().unwrap()
        .participant_ids
        .split(",")
        .into_iter()
        .collect::<HashSet<_>>();

    user_ids.insert(&new_conversation.user_id);

    let v = user_ids.clone().into_iter().map(|s| s.to_string()).collect::<Vec<String>>().join(",");
    let _updated_row = diesel::update(rooms::table.filter(rooms::id.eq(&new_conversation.room_id)))
    .set((rooms::participant_ids.eq(v), rooms::last_message.eq(&new_conversation.message), rooms::name.eq(&user.username)))
    .execute(conn);

    Ok(room)
}

pub fn insert_new_user(conn: &mut SqliteConnection, nm: &str, pn: &str) -> Result<User, DbError> {
    use crate::schema::users::dsl::*;
    let new_user = User {
        id: Uuid::new_v4().to_string(),
        username: nm.to_owned(),
        phone: pn.to_owned(),
        created_at: iso_date(),
    };
    diesel::insert_into(users).values(&new_user).execute(conn)?;
    Ok(new_user)
}

pub fn insert_new_conversation(
    conn: &mut SqliteConnection,
    new: NewConversation,
) -> Result<Conversation, DbError> {
    use crate::schema::conversations::dsl::*;
    let new_conversation = Conversation {
        id: Uuid::new_v4().to_string(),
        user_id: new.user_id,
        room_id: new.room_id,
        content: new.message,
        created_at: iso_date(),
    };
    diesel::insert_into(conversations)
        .values(&new_conversation)
        .execute(conn)?;
    Ok(new_conversation)
}

pub fn get_all_rooms(conn: &mut SqliteConnection) -> Result<Vec<RoomResponse>, DbError> {
    use crate::schema::rooms;
    use crate::schema::users;
    let rooms_data: Vec<Room> = rooms::table.get_results(conn)?;
    let mut ids = HashSet::new();
    let mut rooms_map = HashMap::new();
    let data = rooms_data.to_vec();
    for room in &data {
        let user_ids = room
            .participant_ids
            .split(",")
            .into_iter()
            .collect::<Vec<_>>();
        for id in user_ids.to_vec() {
            ids.insert(id.to_string());
        }
        rooms_map.insert(room.id.to_string(), user_ids.to_vec());
    }
    let ids = ids.into_iter().collect::<Vec<_>>();
    let users_data: Vec<User> = users::table
        .filter(users::id.eq_any(ids))
        .get_results(conn)?;
    let users_map: HashMap<String, User> = HashMap::from_iter(
        users_data
            .into_iter()
            .map(|item| (item.id.to_string(), item)),
    );
    let response_rooms = rooms_data.into_iter().map(|room| {
        let users = rooms_map
            .get(&room.id.to_string())
            .unwrap()
            .into_iter()
            .map(|id| users_map.get(id.to_owned()).unwrap().clone())
            .collect::<Vec<_>>();
        return RoomResponse{ room, users };
    }).collect::<Vec<_>>();
    Ok(response_rooms)
}
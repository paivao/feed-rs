pub struct Group {
    pub id: i64,
    pub name: String,
}

pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub groups: Vec<Group>,
}
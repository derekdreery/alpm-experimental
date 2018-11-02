//! Serde (de)serializers for the alpm database format.

pub mod de;
mod de_error;
pub mod ser;
mod ser_error;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Record {
        name: String,
        age: u16,
        age_diff: i32,
        height: f32,
        friends: Vec<String>,
        best_friend: (String, u16),
    }

    #[test]
    fn it_works() {
        fn check_record(rec: Record) {
            let serialized = ser::to_string(&rec).unwrap();
            let deserialized: Record = de::from_str(&serialized).unwrap();
            assert_eq!(deserialized, rec);
        }
        check_record(Record {
            name: "Me".to_owned(),
            age: 60,
            age_diff: -1,
            height: 3.0,
            friends: vec!["some".into(), "friends".into()],
            best_friend: ("Arthur".into(), 20),
        });
    }
}

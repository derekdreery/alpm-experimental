pub mod de;
mod de_error;
pub mod ser;
mod ser_error;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize)]
    struct NewRecord {
        name: String,
        age: u16,
        age_diff: i32,
        height: f32,
        friends: Vec<String>,
        best_friend: (String, u16)
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Record<'a> {
        name: &'a str,
        age: u16,
        age_diff: i32,
        height: f32,
        friends: Vec<&'a str>,
        best_friend: (&'a str, u16)
    }

    #[test]
    fn it_works() {
        let serialized = ser::to_string(&NewRecord {
            name: "Me".to_owned(),
            age: 60,
            age_diff: -1,
            height: 3.0,
            friends: vec!["some".into(), "friends".into()],
            best_friend: ("Arthur".into(), 20),
        }).unwrap();
        //panic!("{}", serialized);
        let deserialized: Record<'_> = de::from_str(&serialized).unwrap();
        assert_eq!(
            deserialized,
            Record {
                name: "Me",
                age: 60,
                age_diff: -1,
                height: 3.0,
                friends: vec!["some", "friends"],
                best_friend: ("Arthur", 20),
            }
        );
    }
}

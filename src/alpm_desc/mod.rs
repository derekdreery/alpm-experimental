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
        friends: Vec<String>,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct Record<'a> {
        name: &'a str,
        age: u16,
        friends: Vec<&'a str>,
    }

    #[test]
    fn it_works() {
        let serialized = ser::to_string(&NewRecord {
            name: "Me".to_owned(),
            age: 60,
            friends: vec!["some".into(), "friends".into()],
        }).unwrap();
        //panic!("{}", serialized);
        let deserialized: Record<'_> = de::from_str(&serialized).unwrap();
        assert_eq!(
            deserialized,
            Record {
                name: "Me",
                age: 60,
                friends: vec!["some", "friends"],
            }
        );
    }
}

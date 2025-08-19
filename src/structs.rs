/*
Some fields and functions in this file have been commented out as they are not actually used in the code.
They are left in though, as they might be necessary later (and I already coded them anyway)
*/


//
// - - - Structs - - -
//
pub struct McUsername {
    pub name: String,
    pub uuid: String
}

pub struct DcUsername {
    // pub rawid: i64,
    pub pingid: String
}

//
// - - - Implementations - - -
//
impl McUsername {
    pub fn new_from_name(name: &str) -> Self {
        McUsername { 
            name: name.to_string(), 
            uuid: "TEMPORARY-UUID".to_string() //TODO: implement UUID getting
        }
    }

    /* pub fn new_from_uuid(uuid: &str) -> Self {
        McUsername {
            name: "tempname".to_string(),
            uuid: uuid.to_string()
        }
    } */
}

impl DcUsername {
    /* pub fn new_from_rawid(id: i64) -> Self {
        DcUsername {
            rawid: id,
            pingid: format!("<@{}>", id)
        }
    } */

    pub fn new_from_pingid(id: &str) -> Self {
        DcUsername {
            /* rawid: {
                let str_id = id.strip_prefix("<@").unwrap().strip_suffix(">").unwrap();
                let int_id = match str_id.parse::<i64>() {Ok(id) => {id}, Err(_) => {0}};
                if int_id == 0 {panic!("Passed incorrect value, could not parse to i64!")};
                int_id
            }, */
            pingid: id.to_string()
        }
    }

    pub fn try_from_pingid(id: &str) -> Option<Self> {
        if !(id.starts_with("<@") && id.ends_with(">")) {
            return None
        } else { // Id starts with <@ and ends with >
            return Some(DcUsername::new_from_pingid(id))
        }
    }
}
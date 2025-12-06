use std::collections::BTreeMap;
use redis::TypedCommands;

pub enum RedisValue {
    Unknown,
    Hash(BTreeMap<String, String>),
}

pub struct RedisClient {
    conn: redis::Connection,
}

impl RedisClient {
    pub fn new() -> redis::RedisResult<Self> {
        // TODO(lahavs): Have IP and port arguments
        let client = redis::Client::open("redis://127.0.0.1:6379")?;
        let conn = client.get_connection()?;

        Ok(Self {
            conn
        })
    }

    pub fn get_redis_keys(&mut self) -> redis::RedisResult<Vec<String>> {
        let mut keys: Vec<String> = self.conn.scan()?.collect::<redis::RedisResult<Vec<String>>>()?;
        keys.sort();
        Ok(keys)

        // TODO(lahavs): Remove
        /*
        let mut v = Vec::default();
        for i in 1..100000 {
        // for i in 1..100 {
            v.push(format!("hello{i}"));
        }
        v
        */
    }

    pub fn get_redis_value(&mut self, redis_key: &str) -> redis::RedisResult<RedisValue> {
        let redis_type: String = redis::cmd("TYPE").arg(redis_key).query(&mut self.conn)?;

        match redis_type.as_str() {
            "hash" => {
                Ok(RedisValue::Hash(self.conn.hgetall(redis_key)?.into_iter().collect()))
            },
            _ => Ok(RedisValue::Unknown)
        }

        // TODO(lahavs): Remove
        /*
        let id = redis_key.strip_prefix("hello").unwrap();
        return RedisValue::string(format!("the_value{id}"));

        match redis_key {
            "hello1" => RedisValue::string("the_value1".into()),
            "hello2" => RedisValue::hash([
                ("key1".into(), "value1".into()),
                ("key2".into(), "value2".into()),
            ].into()),
            "wow1" => RedisValue::string("the_value3".into()),
            "wow2" => RedisValue::string("the_value4".into()),
            _ => RedisValue::Unknown,
        }
        */
    }
}

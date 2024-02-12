use std::collections::HashMap;

use testcontainers::{core::WaitFor, Image, ImageArgs};

const NAME: &str = "mysql";
const TAG: &str = "8.1";

pub struct Mysql {
    env_vars: HashMap<String, String>,
}

impl Default for Mysql {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("MYSQL_ALLOW_EMPTY_PASSWORD".into(), "yes".into());

        Self { env_vars }
    }
}

impl Image for Mysql {
    type Args = MysqlArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("X Plugin ready for connections. Bind-address"),
            WaitFor::message_on_stderr("/usr/sbin/mysqld: ready for connections."),
            WaitFor::seconds(2),
        ]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[derive(Debug, Clone, Default)]
pub struct MysqlArgs;

impl ImageArgs for MysqlArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(
            vec![
                "--gtid_mode=ON".to_string(),
                "--enforce_gtid_consistency=ON".to_string(),
            ]
            .into_iter(),
        )
    }
}

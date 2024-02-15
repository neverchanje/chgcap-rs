## Compatible versisons

- MariaDB: 11.1, 11.2, 11.3
- MySQL: 5.7.x, 8.0.x, 8.1, 8.2
- Amazon RDS MySQL (5.7 and 8.0)
- Amazon Aurora 

Please file an issue if you wish to have support for any older versions.

## Data types support

## Prerequisites

- Create a replication-specific user with the required permissions:
  ```sql
  CREATE USER <username>@'%' IDENTIFIED WITH mysql_native_password BY 'password';
  GRANT SELECT, REPLICATION CLIENT, REPLICATION SLAVE ON *.* TO <username>@'%';
  ```

- Ensures that `binlog_row_image` is set to `FULL`.
  We assume that all columns in each row are logged, not only the changed columns.
  ```sql
  SHOW VARIABLES LIKE 'binlog_row_image';
  ```
  FULL is the default setting.

- Ensures that `binlog_format` is set to `ROW`.
  To check whether the master server has enabled ROW-based replication, you can use the command:
  ```sql
  SHOW VARIABLES LIKE 'binlog_format';
  ```
  Row-based logging is the default method. See also <https://dev.mysql.com/doc/refman/8.0/en/replication-formats.html>.

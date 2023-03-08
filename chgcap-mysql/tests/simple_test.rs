use chgcap_mysql::{MysqlSource, MysqlSourceConfigBuilder};
use futures::StreamExt;

#[tokio::test]
async fn test_simple() {
    let cfg = MysqlSourceConfigBuilder::default()
        .hostname("127.0.0.1".into())
        .port(3306)
        .username("root".into())
        .password("123456".into())
        .server_id(1)
        .build()
        .unwrap();
    let s = MysqlSource::new(cfg).await.unwrap();
    let stream = s.cdc_stream().await.unwrap();
    stream
        .for_each(|change_result| {
            let change = change_result.unwrap();
            println!("{change:?}");
            std::future::ready(())
        })
        .await;
}

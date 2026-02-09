#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut futures = Vec::new();

    println!("正在发送 15 个并发请求...");

    for i in 1..=15 {
        let client = client.clone();
        futures.push(tokio::spawn(async move {
            let resp = client.get("http://127.0.0.1:3000/slow").send().await;
            match resp {
                Ok(r) => println!("请求 {} 结束，状态码: {}", i, r.status()),
                Err(e) => println!("请求 {} 失败: {}", i, e),
            }
        }));
    }

    // 等待所有任务结束
    for f in futures {
        let _ = f.await;
    }

    Ok(())
}
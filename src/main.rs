use tokio::time::{sleep, Duration};
use tokio::task::spawn;

async fn task_one() {
    println!("task one: start");
    sleep(Duration::from_secs(1)).await;
    println!("task one: done");
}

async fn task_two() {
    println!("task two: start");
    sleep(Duration::from_secs(2)).await;
    println!("task two: done");
}

#[tokio::main]
async fn main() {
    let _ = spawn(async {
        println!("Runtime started...");
    }).await;

    task_one().await;
    task_two().await;
}
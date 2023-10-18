use warp::Filter;

pub fn run_local_server() {
    tokio::task::spawn(async move {
        let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));
        warp::serve(hello).run(([0, 0, 0, 0], 6060)).await;
    });
}

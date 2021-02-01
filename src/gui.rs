use futures::{Stream, StreamExt};
use std::fmt::Debug;

pub async fn run<T>(swarm: &mut T)
where
    T: Stream + Unpin,
    <T as Stream>::Item: Debug,
{
    loop {
        while let Some(event) = swarm.next().await {
            println!("{:?}", event);
        }
    }
}

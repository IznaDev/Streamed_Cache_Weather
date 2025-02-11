# Streamed_Cache_Weather
A rust asynchronous library implementing a streamed cache bounding to a weather API.

In this litte program I use the tokio crate in order to create a streamed cache communicating with a weather API ASYNCHORNOUSLY. 

I used mutex and Arc to handle the datas via different threads, spawn tokio fonction to launch the subscribe and fetch functions in the background, and of course async/await and the very smart trait Future !

I simulated a Weather API. 

Finally, I implemented tests. 

it's a first draft that I'll develop in my next commits. 

To test my code: cargo run test !

Long live Rust ! 

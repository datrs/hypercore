use random_access_memory::RandomAccessMemory;
use {generate_keypair, Feed, Storage};

#[no_mangle]
pub extern "C" fn new_feed() -> *mut Feed<RandomAccessMemory> {
  let keypair = generate_keypair();
  let storage = Storage::new_memory().unwrap();
  let feed = Feed::builder(keypair.public, storage)
    .secret_key(keypair.secret)
    .build()
    .unwrap();

  Box::into_raw(Box::new(feed)) as *mut Feed<RandomAccessMemory>
}

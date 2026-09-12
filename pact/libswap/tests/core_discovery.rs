//! Real JSON-RPC boundary regressions for cached scans and pre-24 Core forks.
use bitcoin::{
    absolute, transaction, Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use libswap::chain::{ChainBackend, CoreRpcBackend};
use libswap::params::BTC_REGTEST;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

struct Fixture {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Fixture {
    fn new(mut reply: impl FnMut(Value) -> Value + Send + 'static) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://test:test@{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let thread = std::thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        continue;
                    }
                    Err(e) => panic!("accept: {e}"),
                };
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                    .unwrap();
                let mut reader = BufReader::new(&stream);
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            length = value.trim().parse().unwrap();
                        }
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let response = reply(serde_json::from_slice(&body).unwrap()).to_string();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.len(),
                    response
                )
                .unwrap();
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
        }
    }
    fn backend(&self) -> CoreRpcBackend {
        CoreRpcBackend::new(&BTC_REGTEST, &self.url).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let joined = self.thread.take().unwrap().join();
        if !std::thread::panicking() {
            joined.unwrap();
        }
    }
}

#[test]
fn scan_cache_returns_answers_invalidates_on_tip_and_rechecks_positive() {
    let stage = Arc::new(AtomicUsize::new(1));
    let scans = Arc::new(AtomicUsize::new(0));
    let (st, calls) = (stage.clone(), scans.clone());
    let txid = "11".repeat(32);
    let server = Fixture::new(move |r| {
        let stage = st.load(Ordering::SeqCst);
        let result = match r["method"].as_str().unwrap() {
            "getbestblockhash" => json!(format!("tip-{stage}")),
            "scantxoutset" => {
                calls.fetch_add(1, Ordering::SeqCst);
                json!({"success":true,"height":stage,"unspents":if stage == 2 {vec![json!({"txid":txid,"vout":0,"amount":0.001,"height":2})]} else {vec![]}})
            }
            "gettxout" => {
                if stage == 2 {
                    json!({"value":0.001,"scriptPubKey":{"hex":"51"},"confirmations":1})
                } else {
                    Value::Null
                }
            }
            other => panic!("unexpected {other}"),
        };
        json!({"result":result,"error":null,"id":"libswap"})
    });
    let backend = Arc::new(server.backend());
    let spk = ScriptBuf::from_bytes(vec![0x51]);
    let other = backend.clone();
    let watch = spk.clone();
    let concurrent = std::thread::spawn(move || other.find_funding(&watch).unwrap());
    assert!(backend.find_funding(&spk).unwrap().is_none());
    assert!(concurrent.join().unwrap().is_none());
    assert_eq!(scans.load(Ordering::SeqCst), 1);
    stage.store(2, Ordering::SeqCst);
    assert!(backend.find_funding(&spk).unwrap().is_some());
    assert!(backend.find_funding(&spk).unwrap().is_some());
    assert_eq!(scans.load(Ordering::SeqCst), 2);
    stage.store(3, Ordering::SeqCst);
    assert!(backend.find_funding(&spk).unwrap().is_none());
    assert_eq!(scans.load(Ordering::SeqCst), 3);
}

fn legacy_mempool_case(size: usize, delay: bool) {
    let op = OutPoint::null();
    let tx = Transaction {
        version: transaction::Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: op,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::from_slice(&[vec![42; 32]]),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(500),
            script_pubkey: ScriptBuf::new(),
        }],
    };
    let raw = bitcoin::consensus::encode::serialize_hex(&tx);
    let server = Fixture::new(move |r| {
        let result = match r["method"].as_str().unwrap() {
            "gettxspendingprevout" => {
                return json!({"result":null,"error":{"code":-32601,"message":"Method not found"},"id":"libswap"})
            }
            "gettxout" => Value::Null,
            "getrawmempool" => json!(vec!["11".repeat(32); size]),
            "getrawtransaction" => {
                if delay {
                    std::thread::sleep(std::time::Duration::from_millis(2100));
                }
                json!({"vin":[]})
            }
            "getblockcount" => json!(100),
            "getblockhash" => json!("block-100"),
            "getblock" => {
                json!({"tx":[{"hex":raw,"vin":[{"txid":op.txid.to_string(),"vout":op.vout}]}]})
            }
            other => panic!("unexpected {other}"),
        };
        json!({"result":result,"error":null,"id":"libswap"})
    });
    let witness = server
        .backend()
        .find_spend_witness(&op, &ScriptBuf::new(), 100)
        .unwrap()
        .unwrap();
    assert_eq!(witness, vec![vec![42; 32]]);
}

#[test]
fn legacy_large_mempool_still_discovers_mined_secret() {
    legacy_mempool_case(200, false);
}

#[test]
fn legacy_slow_mempool_still_reaches_block_scan() {
    legacy_mempool_case(2, true);
}

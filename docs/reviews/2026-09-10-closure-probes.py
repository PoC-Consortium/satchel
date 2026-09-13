"""Adapt the previous isolated probes to test closure, plus Core abstention."""
from pathlib import Path
base = Path(__file__).with_name('2026-09-10-fix-probes.py').read_text()
base = base.replace('assert!(witness_authentic(&spk, value, &invalid, 0));',
                    'assert!(!witness_authentic(&spk, value, &invalid, 0));')
base = base.replace('CONFIRMED: refund branch signed by redeem key accepted as authentic Refund',
                    'FIX VERIFIED: wrong branch signature rejected')
base = base.replace('tip: 100, spend: None };', 'tip: 100, spend: None, final_confs: Some(0) };')
base = base.replace('if height == 0 {0} else {6}', '0')
base = base.replace('assert!(!swap_needs_coin(&record, "btc"));', 'assert!(swap_needs_coin(&record, "btc"));')
base = base.replace('CONFIRMED: coin removal guard misses live swap with actual ChainRef serialization',
                    'FIX VERIFIED: coin guard detects actual ChainRef wire record')
injection = r'''
    #[test]
    fn review_core_abstention_leaves_one_secondary_in_charge() {
        use libswap::chain::MultiBackend;
        let core = MockBackend { history: None, txs: vec![], tip:100, spend:None, final_confs:Some(u64::MAX) };
        let secondary = MockBackend { history: None, txs: vec![], tip:100, spend:None, final_confs:Some(99) };
        let pool = MultiBackend::from_backends(vec![Box::new(core), Box::new(secondary)]).unwrap();
        let depth = pool.tx_confirmations_final("example", None).unwrap();
        assert_eq!(depth, 99);
        println!("RESIDUAL: trusted primary abstains, single secondary accepted at {depth} confirmations");
    }
'''
override = '''        fn tx_confirmations_final(&self, txid: &str, spk: Option<&ScriptBuf>) -> Result<u64> {
            if self.final_confs == Some(u64::MAX) { anyhow::bail!("trusted Core cannot see this tx"); }
            self.tx_confirmations(txid, spk)
        }
        fn tx_confirmations(&self, txid:'''
base = base.replace("pos = source.rfind('}')", "extra += " + repr(injection) + "\n" +
    "source = source.replace('&BTC_REGTEST', 'libswap::registry::get(\"btc\").unwrap().params(Network::Mainnet).unwrap()')\n" +
    "source = source.replace('        fn tx_confirmations(&self, txid:', " + repr(override) + ")\n" +
    "pos = source.rfind('}')")
exec(compile(base, __file__, 'exec'))

"""Recheck the last residual using public-server trust metadata."""
from pathlib import Path
source = Path(__file__).with_name('2026-09-10-closure-probes.py').read_text()
source = source.replace(
    'let depth = pool.tx_confirmations_final("example", None).unwrap();\n        assert_eq!(depth, 99);\n        println!("RESIDUAL: trusted primary abstains, single secondary accepted at {depth} confirmations");',
    'let result = pool.tx_confirmations_final("example", None);\n        assert!(result.is_err());\n        println!("FIX VERIFIED: trusted Core abstention plus one public responder gives no finality verdict: {:?}", result);')
source = source.replace("override = '''        fn tx_confirmations_final", """override = '''        fn view_health(&self) -> Option<std::sync::Arc<libswap::server_health::ServerHealth>> {
            if self.final_confs == Some(99) {
                Some(libswap::server_health::server_health("review-finality", "tcp://mock:1"))
            } else { None }
        }
        fn tx_confirmations_final""")
exec(compile(source, __file__, 'exec'))

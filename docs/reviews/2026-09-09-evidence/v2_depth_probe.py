import sys, inspect
sys.path.insert(0, r'C:\code\pocx\satchel\pact\harness')
sys.path.insert(0, r'C:\code\pocx\satchel\pact\harness\tests')
import swap_v2_adaptor as suite
from framework.node import Harness
source = inspect.getsource(suite.test_adaptor_swap)
source = source.replace('_broadcast_leg_b(bob, h.btc, "bob_btc")', '_broadcast_leg_b(bob, h.btc, "bob_btc", confs=0)')
prefix = source.split('        # Funding outpoints,')[0]
cleanup = source[source.index('    finally:'):]
probe = '''        b_txid, b_vout = fb["body"]["txid"], fb["body"]["vout"]
        before = h.btc.rpc("gettxout", b_txid, b_vout)
        print("REVIEW funding confirmations:", before["confirmations"], "required:", ar["n_b"], flush=True)
        assert before["confirmations"] == 0 and ar["n_b"] > 0
        result = alice.rpc("adaptorredeem", sid)
        print("REVIEW premature redeem accepted; state:", result["record"]["state"], flush=True)
        assert result["record"]["state"] == "redeemed_b"
'''
exec(compile(prefix + probe + cleanup, '<review-v2-depth-probe>', 'exec'), suite.__dict__)
with Harness(use_cache=True) as h:
    suite.test_adaptor_swap(h)

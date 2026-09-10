import sys, inspect
sys.path.insert(0, r'C:\code\pocx\satchel\pact\harness')
sys.path.insert(0, r'C:\code\pocx\satchel\pact\harness\tests')
import swap_v1 as suite
from framework.node import Harness
source = inspect.getsource(suite.test_daemon_autopilot_refund)
prefix = source.split('        events = bob.tick()')[0]
cleanup = source[source.index('    finally:'):]
probe = '''        record = alice.rpc("getswap", sid)
        print("REVIEW own MTP:", h.pocx.rpc("getblockchaininfo")["mediantime"], "refund lock:", record["t1"], flush=True)
        h.btc.stop()
        events = alice.tick()
        print("REVIEW tick with opposite backend down:", [(e["action"], e["detail"]) for e in events], flush=True)
        assert not any(e["action"] == "auto-refund" for e in events)
        result = alice.rpc("refund", sid)
        print("REVIEW direct refund via healthy own chain succeeds:", result["record"]["state"], flush=True)
        tip = h.pocx.rpc("getbestblockhash")
        print("REVIEW getblock verbosity 2 tx has hex:", "hex" in h.pocx.rpc("getblock", tip, 2)["tx"][0], flush=True)
'''
exec(compile(prefix + probe + cleanup, '<review-refund-outage-probe>', 'exec'), suite.__dict__)
with Harness(use_cache=True) as h:
    suite.test_daemon_autopilot_refund(h)

import base64,json,os,pathlib,socket,sqlite3,statistics,subprocess,tempfile,time,urllib.request
root=pathlib.Path(tempfile.mkdtemp(prefix='satchel-code-review-'))
with socket.socket() as sock:
 sock.bind(('127.0.0.1',0)); port=sock.getsockname()[1]
env=dict(os.environ,PACT_DISABLE_KEYRING='1')
log=open(root/'daemon.log','w')
p=subprocess.Popen([r'C:\code\pocx\satchel\pact\target\debug\pactd.exe','--data-dir',str(root),'--network','regtest','--listen',f'127.0.0.1:{port}','--merchants','--tick-secs','0'],stdout=log,stderr=log,env=env,creationflags=0x08000000)
def rpc(method,*params):
 auth=base64.b64encode((root/'.cookie').read_bytes().strip()).decode()
 req=urllib.request.Request(f'http://127.0.0.1:{port}/',json.dumps(dict(jsonrpc='2.0',id=1,method=method,params=params)).encode(),{'Authorization':'Basic '+auth,'Content-Type':'application/json'})
 with urllib.request.urlopen(req,timeout=30) as r: raw=r.read()
 result=json.loads(raw)
 if result.get('error'): raise RuntimeError(result['error'])
 return result.get('result'),len(raw)
try:
 for _ in range(100):
  try: rpc('listmerchants');break
  except Exception:time.sleep(.1)
 a=rpc('createmerchant','review-a')[0]['id']; b=rpc('createmerchant','review-b')[0]['id'];rpc('loadmerchant',a)
 db=root/'merchants'/a/'pact.sqlite'
 base=dict(swap_id='review-swap',role='initiator',state='accepted',created_at=int(time.time()),swap_index=0,chain_a=dict(asset='btcx',network='regtest'),chain_b=dict(asset='btc',network='regtest'),amount_a=1000000,amount_b=1000000,hash_h='11'*32,t1=2000000000,t2=1999900000,n_a=1,n_b=1,alice_refund_pubkey_a='02'+'11'*32,alice_redeem_pubkey_b='02'+'22'*32,adopted=True,settled=False)
 def insert(table,record):
  with sqlite3.connect(db) as c:c.execute(f'INSERT OR REPLACE INTO {table}(swap_id,record) VALUES (?,?)',(record['swap_id'],json.dumps(record)))
 insert('swaps',base)
 try:rpc('loadmerchant',b);print('UNEXPECTED: live v1 load allowed')
 except RuntimeError:print('CONTROL: loadmerchant rejects active v1',flush=True)
 print('BUG: createmerchant switches away from active v1:',rpc('createmerchant','review-c')[0],flush=True)
 rpc('loadmerchant',a)
 with sqlite3.connect(db) as c:c.execute('DELETE FROM swaps')
 v2=dict(base,state='signed',adaptor_point='02'+'33'*32,alice_swap_a='02'+'11'*32,alice_swap_b='02'+'22'*32,alice_refund_a='44'*32,redeem_feerate_a=1,redeem_feerate_b=1,funding_a_txid='55'*32,funding_a_vout=0)
 insert('adaptor_swaps',v2)
 assert len(rpc('listadaptorswaps')[0])==1
 print('BUG: loadmerchant switches away from signed v2:',rpc('loadmerchant',b)[0],flush=True)
 rpc('loadmerchant',a)
 with sqlite3.connect(db) as c:c.execute('DELETE FROM adaptor_swaps')
 insert('swaps',dict(base,state='completed',settled=False))
 print('BUG: loadmerchant switches away from unsettled completed v1:',rpc('loadmerchant',b)[0],flush=True)
 rpc('loadmerchant',a)
 for n in (0,1000,10000):
  with sqlite3.connect(db) as c:
   c.execute('DELETE FROM swaps')
   for i in range(n):
    rec=dict(base,swap_id=f'{i:016x}',state='completed',settled=True,refund_tx_hex='00'*250,final_tx_hex='00'*250)
    c.execute('INSERT INTO swaps VALUES (?,?)',(rec['swap_id'],json.dumps(rec)))
  for method in ('tick','listswaps'):
   rpc(method)
   durations=[];size=0
   for _ in range(5):
    start=time.perf_counter(); _,size=rpc(method);durations.append((time.perf_counter()-start)*1000)
   print('PERF',json.dumps(dict(rows=n,method=method,median_ms=round(statistics.median(durations),2),max_ms=round(max(durations),2),response_bytes=size)),flush=True)
finally:
 try:rpc('stop')
 except Exception:pass
 try:p.wait(timeout=10)
 except subprocess.TimeoutExpired:p.kill();p.wait()
 log.close()
 print('ARTIFACTS',root,flush=True)

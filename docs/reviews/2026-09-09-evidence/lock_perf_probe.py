import pathlib
folder=pathlib.Path(__file__).parent
src=(folder/'merchant_perf_probe.py').read_text()
prefix=src.split('\ntry:\n for _')[0]
prefix=prefix.replace("'--tick-secs','0'", "'--tick-secs','0','--board-url',f'http://127.0.0.1:{server.server_port}'")
head='''import threading,http.server,concurrent.futures,time
entered=threading.Event()
class Handler(http.server.BaseHTTPRequestHandler):
 def do_POST(self):
  self.rfile.read(int(self.headers.get('Content-Length','0')))
  entered.set();time.sleep(2)
  payload=b'{"messages":[]}'
  self.send_response(200);self.send_header('Content-Length',str(len(payload)));self.end_headers();self.wfile.write(payload)
 def log_message(self,*args):pass
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
'''
body='''
try:
 for _ in range(100):
  try: rpc('listmerchants');break
  except Exception:time.sleep(.1)
 rpc('createmerchant','latency-probe');rpc('createseed')
 with concurrent.futures.ThreadPoolExecutor() as pool:
  future=pool.submit(rpc,'tick')
  assert entered.wait(10)
  t=time.perf_counter();rpc('listswaps');elapsed=time.perf_counter()-t
  print('PERF empty listswaps blocked by 2-second relay:',round(elapsed,3),'seconds',flush=True)
  future.result()
finally:
 try:rpc('stop')
 except Exception:pass
 try:p.wait(timeout=10)
 except subprocess.TimeoutExpired:p.kill();p.wait()
 log.close();server.shutdown()
 print('ARTIFACTS',root,flush=True)
'''
exec(compile(head+prefix+body,'<lock-probe>','exec'))

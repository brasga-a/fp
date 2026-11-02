#!/usr/bin/env python3
"""Smoke PTY do gerenciador, somente diretórios temporários, sem Docker."""
import errno,fcntl,os,pty,select,struct,subprocess,sys,tempfile,termios,time
from pathlib import Path
binary=Path(sys.argv[1] if len(sys.argv)>1 else 'target/release/fp').resolve()
def scenario(steps,code,initialized=False):
 with tempfile.TemporaryDirectory(prefix='fp-db-pty-') as directory:
  root=Path(directory)
  master,slave=pty.openpty()
  fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',45,140,0,0))
  env={**os.environ,'HOME':directory,'XDG_CONFIG_HOME':str(root/'c'),'XDG_DATA_HOME':str(root/'d'),'XDG_STATE_HOME':str(root/'s'),'TERM':'xterm-256color','PATH':'/nonexistent'}
  proc=subprocess.Popen([str(binary),'db','--visual'],stdin=slave,stdout=slave,stderr=slave,start_new_session=True,env=env)
  os.close(slave);pending=bytearray();transcript=bytearray()
  def read():
   if select.select([master],[],[],0.1)[0]:
    try:data=os.read(master,65536)
    except OSError as e:
     if e.errno==errno.EIO:return
     raise
    pending.extend(data);transcript.extend(data)
  try:
   for text,keys in steps:
    deadline=time.monotonic()+8
    while text.encode() not in pending:
     read()
     assert time.monotonic()<deadline and proc.poll() is None,transcript.decode(errors='replace')
    pending.clear();time.sleep(.08);os.write(master,keys)
   deadline=time.monotonic()+8
   while proc.poll() is None and time.monotonic()<deadline:read()
   assert proc.poll()==code,transcript.decode(errors='replace')
   assert (root/'c/fp/databases.json').exists()==initialized
   assert not (root/'d/fp/example.db').exists()
   assert b'\x1b[?1049h' not in transcript
  finally:
   if proc.poll() is None:proc.kill()
   proc.wait();os.close(master)
down=b'\x1b[B'
scenario([('Operação',b'\x1b')],130)
scenario([('Operação',b'\x03')],130)
scenario([('Operação',down*3+b'\r'),('Operação',b'\x1b')],130)
scenario([('Operação',down*7+b'\r'),('Confirmar operação',b'\r'),('Operação',b'\x1b')],130)
scenario([('Operação',down*7+b'\r'),('Confirmar operação',b'y'),('Operação',b'\x1b')],130,True)
print('5 cenários PTY de bancos passaram: teclado, consulta, cancelamento e confirmação de init.')

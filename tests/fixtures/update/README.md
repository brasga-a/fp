# Fixtures de atualização

`release.tar.gz` contém um ELF Linux x86_64 mínimo compilado de `stub.S`.
Ele apenas escreve `fp 0.2.0` e termina com sucesso. `SHA256SUMS` é o SHA-256
real do archive; não depende de uma release pública. A versão fixa é somente
um dado de teste, independente da versão real do pacote.

O ELF pode ser recompilado para inspeção com:

```sh
cc -nostdlib -static -Wl,--build-id=none -s stub.S -o /tmp/fp-update-test-stub
```

O archive é um tar USTAR com um único arquivo regular `fp`, modo 0755,
UID/GID/mtime zero, comprimido com gzip (mtime zero). Nenhum compilador é
executado pelos testes automatizados: usam o archive já incluído.

`invalid.tar.gz` contém um script que imprime a mesma versão, para garantir
que o validador recuse formatos diferentes de ELF antes da execução.
`INVALID_SHA256SUMS` permite passar a integridade e chegar à validação de formato.
Os testes executam e instalam somente fixtures em diretórios temporários.

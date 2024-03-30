
corrupt_merkle - byte at offset 41 changed from 2b to 2c

get value, seek to position 40 then read one byte: xxd -seek 40 -l 1 -ps corrupt_merkle.bin -
write different value, seek to position 40 then write one byte: printf "28: 2c" | xxd -r - corrupt_merkle.bin
compare files: cmp -l ../0000000000000000000988036522057056727ae85ad7cea92b2198418c9bb8f7.bin corrupt_merkle.bin

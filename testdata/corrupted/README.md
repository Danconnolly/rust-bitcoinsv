# Corrupted Block Files

* too_small.bin - removed last byte from file
* too_long.bin - added one byte to the end of the file
* corrupt_merkle.bin - byte at offset 41 changed from 53 to 54  (all numbers decimal)

## To produce corrupted file

* get the value - seek to position 40 then read one byte: `xxd -seek 40 -l 1 -ps corrupt_merkle.bin -`
* write different value - seek to position 40 then write one byte: `printf "28: 2c" | xxd -r - corrupt_merkle.bin`
* compare files: `cmp -l ../0000000000000000000988036522057056727ae85ad7cea92b2198418c9bb8f7.bin corrupt_merkle.bin`

# Transaction

Right now we start a transaction then pass a reference of the transaction to the inode layer
to do modification to the filesystem.

## TODO:
* This should be one operation. We start a transaction then use the transaction to access
the inode layer.

# Numbers
| Function | Cycles |
| -------- | ------ |
| search buffer | 500 |
| recycle buffer | 800 |
| bdev.read 4k | 6000 |
| memcpy 512 | 2000 |
| redox memcpy 512 | 700 |
| block map | 2400 |
| bread | 600 |
| brelse | 400 |
| iguard read | 3500 |
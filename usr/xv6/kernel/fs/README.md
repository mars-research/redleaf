# Transaction

Right now we start a transaction then pass a reference of the transaction to the inode layer
to do modification to the filesystem.

## TODO:
* This should be one operation. We start a transaction then use the transaction to access
the inode layer.